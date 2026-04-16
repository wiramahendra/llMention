use anyhow::{bail, Result};
use async_trait::async_trait;
use reqwest::Client;
use serde_json::{json, Value};

use super::LlmProvider;
use crate::config::ProviderConfig;

pub struct AnthropicProvider {
    client: Client,
    config: ProviderConfig,
}

impl AnthropicProvider {
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
impl LlmProvider for AnthropicProvider {
    fn name(&self) -> &str {
        "anthropic"
    }

    async fn query_with_system(&self, system: Option<&str>, prompt: &str) -> Result<String> {
        let mut body = json!({
            "model": self.config.model,
            "max_tokens": 1024,
            "temperature": self.config.temperature,
            "messages": [{"role": "user", "content": prompt}]
        });
        if let Some(sys) = system {
            body["system"] = json!(sys);
        }

        let resp = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.config.api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            bail!("Anthropic error {}: {}", status, text);
        }

        let json: Value = resp.json().await?;
        Ok(json["content"][0]["text"]
            .as_str()
            .unwrap_or("")
            .to_string())
    }
}
