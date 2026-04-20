use anyhow::{Context, Result};
use dirs::home_dir;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct Config {
    #[serde(default)]
    pub providers: ProvidersConfig,
    #[serde(default)]
    pub defaults: DefaultsConfig,
    #[serde(default)]
    pub judge: JudgeConfig,
    #[serde(default)]
    pub plugins: PluginsConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct ProvidersConfig {
    pub openai: Option<ProviderConfig>,
    pub anthropic: Option<ProviderConfig>,
    pub gemini: Option<ProviderConfig>,
    pub xai: Option<ProviderConfig>,
    pub perplexity: Option<ProviderConfig>,
    pub ollama: Option<OllamaConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProviderConfig {
    pub api_key: String,
    pub model: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub temperature: f32,
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OllamaConfig {
    #[serde(default = "default_ollama_url")]
    pub base_url: String,
    pub model: String,
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub temperature: f32,
}

/// Optional LLM-as-judge config. When enabled (or --judge CLI flag used),
/// a local model re-evaluates each response for higher-accuracy results.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct JudgeConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_ollama_url")]
    pub base_url: String,
    #[serde(default = "default_judge_model")]
    pub model: String,
}

impl Default for JudgeConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            base_url: default_ollama_url(),
            model: default_judge_model(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DefaultsConfig {
    #[serde(default = "default_days")]
    pub days: u32,
    #[serde(default = "default_concurrency")]
    pub concurrency: usize,
}

impl Default for DefaultsConfig {
    fn default() -> Self {
        Self {
            days: default_days(),
            concurrency: default_concurrency(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PluginsConfig {
    /// Names of installed plugins that are active by default.
    #[serde(default)]
    pub enabled_plugins: Vec<String>,
    /// Reserved for future auto-update support.
    #[serde(default)]
    pub auto_update: bool,
}

impl Default for PluginsConfig {
    fn default() -> Self {
        Self { enabled_plugins: Vec::new(), auto_update: false }
    }
}

fn default_true() -> bool { true }
fn default_ollama_url() -> String { "http://localhost:11434".to_string() }
fn default_judge_model() -> String { "llama3.2".to_string() }
fn default_days() -> u32 { 7 }
fn default_concurrency() -> usize { 5 }
fn default_timeout() -> u64 { 30 }

impl Config {
    pub fn load() -> Result<Self> {
        let path = config_path();
        if !path.exists() {
            return Ok(Self::default());
        }
        let contents = std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read config at {}", path.display()))?;
        toml::from_str(&contents).with_context(|| "Failed to parse config.toml")
    }

    pub fn config_dir() -> PathBuf {
        home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".llmention")
    }

    /// Creates ~/.llmention/ and ~/.llmention/cache/ if they don't exist.
    /// Returns (dir, is_first_run).
    pub fn ensure_dir() -> Result<(PathBuf, bool)> {
        let dir = Self::config_dir();
        let is_new = !dir.exists();
        std::fs::create_dir_all(&dir)
            .with_context(|| format!("Failed to create config dir {}", dir.display()))?;
        std::fs::create_dir_all(dir.join("cache"))?;
        Ok((dir, is_new))
    }
}

pub fn config_path() -> PathBuf {
    Config::config_dir().join("config.toml")
}

pub const EXAMPLE_CONFIG: &str = r#"# LLMention configuration
# ~/.llmention/config.toml

# Cloud providers — set enabled = true and add your API key.
# temperature = 0 gives deterministic, cacheable results (recommended).

[providers.openai]
api_key     = "sk-..."
model       = "gpt-4o-mini"
enabled     = true
temperature = 0

[providers.anthropic]
api_key     = "sk-ant-..."
model       = "claude-3-5-haiku-20241022"
enabled     = true
temperature = 0

[providers.gemini]
api_key     = "AIza..."
model       = "gemini-2.0-flash"
enabled     = false
temperature = 0

[providers.xai]
api_key     = "xai-..."
model       = "grok-2-latest"
enabled     = false
temperature = 0

[providers.perplexity]
api_key   = "pplx-..."
model     = "sonar"
enabled   = false
temperature = 0

# Local model via Ollama — no API key needed.
# Install: https://ollama.com  then: ollama pull llama3.2
[providers.ollama]
base_url  = "http://localhost:11434"
model     = "llama3.2"
enabled   = false

# LLM-as-judge: re-evaluates each response with a local model for
# higher-accuracy mention/sentiment detection. Uses Ollama.
# Enable with: llmention track ... --judge
[judge]
enabled   = false
base_url  = "http://localhost:11434"
model     = "llama3.2"

[defaults]
days        = 7
concurrency = 5
"#;
