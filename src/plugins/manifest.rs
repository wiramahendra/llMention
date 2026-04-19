use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PluginManifest {
    pub meta: PluginMeta,
    #[serde(default)]
    pub templates: PluginTemplates,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PluginMeta {
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct PluginTemplates {
    /// Path to generate system prompt template, relative to plugin directory.
    pub generate: Option<String>,
    /// Path to discover user prompt template, relative to plugin directory.
    pub discover: Option<String>,
}
