use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::PathBuf;

#[derive(Serialize, Deserialize)]
struct CacheEntry {
    response: String,
    cached_at: DateTime<Utc>,
}

pub struct Cache {
    dir: PathBuf,
}

impl Cache {
    pub fn new(base_dir: &PathBuf) -> Result<Self> {
        let dir = base_dir.join("cache");
        std::fs::create_dir_all(&dir)?;
        Ok(Self { dir })
    }

    fn key(model: &str, prompt: &str) -> String {
        let mut h = Sha256::new();
        h.update(model.as_bytes());
        h.update(b"|");
        h.update(prompt.as_bytes());
        hex::encode(h.finalize())
    }

    pub fn get(&self, model: &str, prompt: &str) -> Option<String> {
        let path = self.dir.join(format!("{}.json", Self::key(model, prompt)));
        let contents = std::fs::read_to_string(path).ok()?;
        let entry: CacheEntry = serde_json::from_str(&contents).ok()?;
        let age = Utc::now().signed_duration_since(entry.cached_at);
        (age.num_hours() < 24).then_some(entry.response)
    }

    pub fn set(&self, model: &str, prompt: &str, response: &str) -> Result<()> {
        let path = self.dir.join(format!("{}.json", Self::key(model, prompt)));
        let entry = CacheEntry {
            response: response.to_string(),
            cached_at: Utc::now(),
        };
        std::fs::write(path, serde_json::to_string(&entry)?)?;
        Ok(())
    }
}
