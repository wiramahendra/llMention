use std::path::PathBuf;

use super::manifest::PluginManifest;

#[derive(Debug, Clone)]
pub struct Plugin {
    pub manifest: PluginManifest,
    pub dir: PathBuf,
}

impl Plugin {
    pub fn generate_template(&self) -> Option<String> {
        let rel = self.manifest.templates.generate.as_ref()?;
        std::fs::read_to_string(self.dir.join(rel)).ok()
    }

    pub fn discover_template(&self) -> Option<String> {
        let rel = self.manifest.templates.discover.as_ref()?;
        std::fs::read_to_string(self.dir.join(rel)).ok()
    }
}

/// Scan `<config_dir>/plugins/` and return all valid plugins.
pub fn discover_plugins(config_dir: &PathBuf) -> Vec<Plugin> {
    let plugins_dir = config_dir.join("plugins");
    if !plugins_dir.exists() {
        return Vec::new();
    }
    let Ok(entries) = std::fs::read_dir(&plugins_dir) else {
        return Vec::new();
    };
    let mut plugins = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let manifest_path = path.join("plugin.toml");
        if !manifest_path.exists() {
            continue;
        }
        let Ok(content) = std::fs::read_to_string(&manifest_path) else {
            continue;
        };
        let Ok(manifest) = toml::from_str::<PluginManifest>(&content) else {
            continue;
        };
        plugins.push(Plugin { manifest, dir: path });
    }
    plugins
}

/// Find a specific installed plugin by name.
pub fn find_plugin(config_dir: &PathBuf, name: &str) -> Option<Plugin> {
    discover_plugins(config_dir)
        .into_iter()
        .find(|p| p.manifest.meta.name == name)
}
