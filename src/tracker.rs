use anyhow::Result;
use chrono::Utc;
use colored::Colorize;
use std::sync::Arc;
use tokio::sync::Semaphore;

use crate::{
    cache::Cache,
    config::Config,
    parser,
    providers::{
        anthropic::AnthropicProvider, ollama::OllamaProvider, openai::OpenAiProvider,
        xai::XaiProvider, LlmProvider,
    },
    storage::Storage,
    types::{MentionResult, TrackSummary},
};

pub fn build_providers(config: &Config) -> Vec<Arc<dyn LlmProvider>> {
    let mut v: Vec<Arc<dyn LlmProvider>> = Vec::new();
    if let Some(c) = &config.providers.openai {
        if c.enabled {
            v.push(Arc::new(OpenAiProvider::new(c.clone())));
        }
    }
    if let Some(c) = &config.providers.anthropic {
        if c.enabled {
            v.push(Arc::new(AnthropicProvider::new(c.clone())));
        }
    }
    if let Some(c) = &config.providers.xai {
        if c.enabled {
            v.push(Arc::new(XaiProvider::new(c.clone())));
        }
    }
    if let Some(c) = &config.providers.ollama {
        if c.enabled {
            v.push(Arc::new(OllamaProvider::new(c.clone())));
        }
    }
    v
}

pub fn build_providers_filtered(
    config: &Config,
    filter: Option<&str>,
) -> Vec<Arc<dyn LlmProvider>> {
    let all = build_providers(config);
    match filter {
        None => all,
        Some(f) => {
            let names: Vec<&str> = f.split(',').map(str::trim).collect();
            all.into_iter().filter(|p| names.contains(&p.name())).collect()
        }
    }
}

pub async fn run_track(
    domain: &str,
    prompts: Vec<String>,
    providers: Vec<Arc<dyn LlmProvider>>,
    storage: &Storage,
    cache: &Cache,
) -> Result<TrackSummary> {
    let sem = Arc::new(Semaphore::new(4));

    // Separate cache hits from live queries
    let mut results: Vec<MentionResult> = Vec::new();
    let mut handles: Vec<tokio::task::JoinHandle<Result<(String, String, String, String)>>> =
        Vec::new();

    for provider in &providers {
        for prompt in &prompts {
            let model = provider.name().to_string();

            if let Some(cached) = cache.get(&model, prompt) {
                let parsed = parser::parse_response(domain, &cached);
                eprintln!(
                    "  {} [cached] [{}] {}",
                    "✓".green(),
                    model.cyan(),
                    prompt.dimmed()
                );
                results.push(MentionResult {
                    domain: domain.to_string(),
                    prompt: prompt.clone(),
                    model,
                    timestamp: Utc::now(),
                    mentioned: parsed.mentioned,
                    cited: parsed.cited,
                    position: parsed.position,
                    sentiment: parsed.sentiment,
                    snippet: parsed.snippet,
                    raw_response: cached,
                });
                continue;
            }

            let provider = Arc::clone(provider);
            let prompt = prompt.clone();
            let sem = Arc::clone(&sem);
            handles.push(tokio::spawn(async move {
                let _permit = sem.acquire().await.unwrap();
                let response = provider.query(&prompt).await?;
                Ok((model, prompt, response, String::new()))
            }));
        }
    }

    for handle in handles {
        match handle.await? {
            Ok((model, prompt, response, _)) => {
                let parsed = parser::parse_response(domain, &response);
                let icon = if parsed.mentioned {
                    "✓".green()
                } else {
                    "–".dimmed()
                };
                eprintln!("  {} [{}] {}", icon, model.cyan(), prompt.dimmed());
                let _ = cache.set(&model, &prompt, &response);
                results.push(MentionResult {
                    domain: domain.to_string(),
                    prompt,
                    model,
                    timestamp: Utc::now(),
                    mentioned: parsed.mentioned,
                    cited: parsed.cited,
                    position: parsed.position,
                    sentiment: parsed.sentiment,
                    snippet: parsed.snippet,
                    raw_response: response,
                });
            }
            Err(e) => eprintln!("  {} query failed: {}", "✗".red(), e),
        }
    }

    for r in &results {
        if let Err(e) = storage.insert(r) {
            eprintln!("  {} failed to persist result: {}", "!".yellow(), e);
        }
    }

    let mention_count = results.iter().filter(|r| r.mentioned).count();
    let citation_count = results.iter().filter(|r| r.cited).count();
    let mut models_with_mention: Vec<String> = results
        .iter()
        .filter(|r| r.mentioned)
        .map(|r| r.model.clone())
        .collect();
    models_with_mention.sort();
    models_with_mention.dedup();

    Ok(TrackSummary {
        domain: domain.to_string(),
        total_queries: results.len(),
        mention_count,
        citation_count,
        models_with_mention,
        results,
    })
}
