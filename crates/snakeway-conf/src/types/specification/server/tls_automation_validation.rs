use crate::types::{AcmeServerSpec, CertStoreSpec, Origin, TlsAutomationSpec};
use crate::validation::validator::validate_cert_pem;
use crate::validation::{RangeConstraint, ValidateSpec, validate_range_field};
use crate::validation::{ValidationReport, range_constraint};
use nix::NixPath;

range_constraint!(RENEW_WITHIN_DAYS, u64, min: 7, max: 30, units: "days");

impl ValidateSpec for TlsAutomationSpec {
    fn validate(&self, origin: &Origin, report: &mut ValidationReport) {
        self.acme.validate(origin, report);

        validate_range_field!(RENEW_WITHIN_DAYS, self.renew_within_days, report, origin);

        self.cert_store.validate(origin, report);
    }
}

impl ValidateSpec for AcmeServerSpec {
    fn validate(&self, origin: &Origin, report: &mut ValidationReport) {
        if self.directory_url.is_empty() {
            report.server_tls_acme_directory_url_cannot_be_empty(origin);
        } else if !self.directory_url.starts_with("https://") {
            report.server_tls_acme_directory_url_must_be_https(origin);
        }

        if self.contact_email.is_empty() {
            report.server_tls_acme_contact_email_cannot_be_empty(origin);
        }

        if let Some(ca_file) = &self.ca_file
            && let Err(e) = validate_cert_pem(ca_file)
        {
            report.server_tls_acme_ca_file_invalid(ca_file, &e, origin);
        }

        if !self.data_dir.is_dir() {
            report.server_tls_acme_data_dir_is_invalid(&self.data_dir, origin);
        }
    }
}

impl ValidateSpec for CertStoreSpec {
    fn validate(&self, origin: &Origin, report: &mut ValidationReport) {
        match self {
            CertStoreSpec::Filesystem { cert_dir } => {
                if cert_dir.is_empty() {
                    report.server_tls_filesystem_cert_store_must_have_a_cert_directory(origin);
                }
            }
            CertStoreSpec::Memory => {}
        }
    }
}
