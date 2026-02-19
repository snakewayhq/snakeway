#[async_trait::async_trait]
pub trait ChallengeExecutor: Send + Sync {
    async fn prepare(&self, domain: &str) -> Result<(), std::io::Error>;

    async fn cleanup(&self, domain: &str);
}
