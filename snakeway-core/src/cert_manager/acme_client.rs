use instant_acme::{Account, AccountCredentials, NewAccount};
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AcmeClientError {
    #[error("cannot read acme account file: {0}")]
    ReadAcmeAccountFile(String),

    #[error("cannot deserialize existing acme account: {0}")]
    DeserializeAcmeAccountFile(String),

    #[error("cannot restore existing acme account: {0}")]
    RestoreAcmeAccount(String),

    #[error("cannot serialize acme account file: {0}")]
    SerializeAcmeAccountFile(String),

    #[error("cannot create acme account: {0}")]
    CreateAcmeAccount(String),

    #[error("cannot write acme account file: {0}")]
    WriteAcmeAccountFile(String),
}

pub struct AcmeClient {
    pub account: Account,
}

impl AcmeClient {
    pub async fn load_or_create(
        directory_url: String,
        order_dir: PathBuf,
        contact_email: Vec<String>,
        ca_file: &Option<PathBuf>,
    ) -> Result<Self, AcmeClientError> {
        let account_path = order_dir.join("acme_account.json");

        let account_builder = ca_file
            .as_ref()
            .map(Account::builder_with_root)
            .unwrap_or_else(Account::builder);

        // Restore existing account
        if account_path.exists() {
            let bytes = fs::read(&account_path)
                .map_err(|e| AcmeClientError::ReadAcmeAccountFile(e.to_string()))?;

            let creds: AccountCredentials = serde_json::from_slice(&bytes)
                .map_err(|e| AcmeClientError::DeserializeAcmeAccountFile(e.to_string()))?;

            let account = account_builder
                .map_err(|e| AcmeClientError::RestoreAcmeAccount(e.to_string()))?
                .from_credentials(creds)
                .await
                .map_err(|e| AcmeClientError::RestoreAcmeAccount(e.to_string()))?;

            return Ok(Self { account });
        }

        // Collect contacts
        let contact_uris: Vec<String> = contact_email
            .iter()
            .map(|email| format!("mailto:{email}"))
            .collect();

        let contact_refs: Vec<&str> = contact_uris.iter().map(|s| s.as_str()).collect();

        let (account, credentials) = account_builder
            .map_err(|e| AcmeClientError::CreateAcmeAccount(e.to_string()))?
            .create(
                &NewAccount {
                    contact: &contact_refs,
                    terms_of_service_agreed: true,
                    only_return_existing: false,
                },
                directory_url,
                None,
            )
            .await
            .map_err(|e| AcmeClientError::CreateAcmeAccount(e.to_string()))?;

        // Persist credentials
        // This is the durable ACME identity account file.
        let serialized = serde_json::to_vec_pretty(&credentials)
            .map_err(|e| AcmeClientError::SerializeAcmeAccountFile(e.to_string()))?;

        atomic_write(&account_path, &serialized)
            .map_err(|e| AcmeClientError::WriteAcmeAccountFile(e.to_string()))?;

        Ok(Self { account })
    }
}

fn atomic_write(path: &Path, data: &[u8]) -> anyhow::Result<()> {
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, data)?;
    fs::rename(tmp, path)?;
    Ok(())
}
