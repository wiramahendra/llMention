pub mod loader;
pub mod manifest;

pub use loader::{discover_plugins, find_plugin, Plugin};
pub use manifest::PluginManifest;
