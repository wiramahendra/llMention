use anyhow::{bail, Result};
use async_trait::async_trait;
use reqwest::Client;
use serde_json::{json, Value};

use super::LlmProvider;
use crate::config::ProviderConfig;

pub struct PerplexityProvider {
    client: Client,
    config: ProviderConfig,
}

impl PerplexityProvider {
    pub fn new(config: ProviderConfig) -> Self {
        Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(config.timeout_secs))
                .build()
                .unwrap_or_default(),
            config,
        }
    }
}

#[async_trait]
impl LlmProvider for PerplexityProvider {
    fn name(&self) -> &str {
        "perplexity"
    }

    async fn query_with_system(&self, system: Option<&str>, prompt: &str) -> Result<String> {
        let mut messages = vec![];
        if let Some(sys) = system {
            messages.push(json!({"role": "system", "content": sys}));
        }
        messages.push(json!({"role": "user", "content": prompt}));

        let body = json!({
            "model": self.config.model,
            "temperature": self.config.temperature,
            "messages": messages
        });

        let resp = self
            .client
            .post("https://api.perplexity.ai/chat/completions")
            .bearer_auth(&self.config.api_key)
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            bail!("Perplexity error {}: {}", status, text);
        }

        let json: Value = resp.json().await?;
        Ok(json["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string())
    }
}
