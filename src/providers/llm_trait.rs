use anyhow::Result;
use async_trait::async_trait;

/// Core abstraction for every LLM backend.
/// Implement `query_with_system`; `query` delegates to it with no system prompt.
#[async_trait]
pub trait LlmProvider: Send + Sync {
    fn name(&self) -> &str;

    async fn query(&self, prompt: &str) -> Result<String> {
        self.query_with_system(None, prompt).await
    }

    async fn query_with_system(&self, system: Option<&str>, prompt: &str) -> Result<String>;
}
