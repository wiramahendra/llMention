use anyhow::Result;
use std::sync::Arc;

use crate::{geo::prompts, providers::LlmProvider};

pub struct EvalResult {
    pub model: String,
    pub would_cite: bool,
    pub confidence: f64,
    pub reason: Option<String>,
}

pub struct EvalDelta {
    /// Baseline: LLM responses to the raw query without any document context.
    pub before: Vec<EvalResult>,
    /// After generating: would the LLMs cite the new document?
    pub after: Vec<EvalResult>,
}

impl EvalDelta {
    pub fn before_rate(&self) -> f64 {
        cite_rate(&self.before)
    }

    pub fn after_rate(&self) -> f64 {
        cite_rate(&self.after)
    }

    pub fn delta(&self) -> f64 {
        self.after_rate() - self.before_rate()
    }
}

fn cite_rate(results: &[EvalResult]) -> f64 {
    if results.is_empty() {
        return 0.0;
    }
    let cited = results.iter().filter(|r| r.would_cite).count();
    cited as f64 / results.len() as f64 * 100.0
}

/// Score a single piece of content across all providers (no baseline, just after).
/// Returns the raw eval results for use by the optimize agent.
pub async fn score_content(
    user_prompt: &str,
    content: &str,
    providers: &[Arc<dyn LlmProvider>],
) -> Result<Vec<EvalResult>> {
    Ok(run_with_content(providers, user_prompt, content).await)
}

pub async fn evaluate_content(
    user_prompt: &str,
    generated_content: &str,
    providers: &[Arc<dyn LlmProvider>],
) -> Result<EvalDelta> {
    let (before, after) = tokio::join!(
        run_baseline(providers, user_prompt),
        run_with_content(providers, user_prompt, generated_content),
    );
    Ok(EvalDelta { before, after })
}

/// Baseline: raw query with no document context.
async fn run_baseline(providers: &[Arc<dyn LlmProvider>], prompt: &str) -> Vec<EvalResult> {
    let handles: Vec<_> = providers
        .iter()
        .map(|p| {
            let p = Arc::clone(p);
            let baseline_q = format!(
                "Without any additional context, can you answer this query from your training data? \
                Query: \"{}\"\n\nRespond ONLY with JSON: \
                {{\"would_cite\": true, \"confidence\": 0.0, \"reason\": \"one sentence\"}}",
                prompt
            );
            tokio::spawn(async move {
                let name = p.name().to_string();
                let result = p.query(&baseline_q).await;
                (name, result)
            })
        })
        .collect();

    collect_eval_results(handles).await
}

/// After: query whether the LLM would cite the generated document.
async fn run_with_content(
    providers: &[Arc<dyn LlmProvider>],
    prompt: &str,
    content: &str,
) -> Vec<EvalResult> {
    let handles: Vec<_> = providers
        .iter()
        .map(|p| {
            let p = Arc::clone(p);
            let q = prompts::build_evaluate_user_prompt(prompt, content);
            tokio::spawn(async move {
                let name = p.name().to_string();
                let result = p.query(&q).await;
                (name, result)
            })
        })
        .collect();

    collect_eval_results(handles).await
}

async fn collect_eval_results(
    handles: Vec<tokio::task::JoinHandle<(String, anyhow::Result<String>)>>,
) -> Vec<EvalResult> {
    let mut results = Vec::new();
    for handle in handles {
        match handle.await {
            Ok((model, Ok(response))) => {
                let (would_cite, confidence, reason) = parse_eval_json(&response);
                results.push(EvalResult { model, would_cite, confidence, reason });
            }
            Ok((model, Err(e))) => {
                eprintln!("  [{}] eval error: {}", model, e);
            }
            Err(e) => eprintln!("  eval task panicked: {}", e),
        }
    }
    results
}

fn parse_eval_json(response: &str) -> (bool, f64, Option<String>) {
    let json_str = extract_json(response);
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&json_str) {
        let would_cite = v["would_cite"].as_bool().unwrap_or(false);
        let confidence = v["confidence"].as_f64().unwrap_or(0.5);
        let reason = v["reason"].as_str().map(String::from);
        return (would_cite, confidence, reason);
    }
    // Fallback: look for "true" in response
    let lower = response.to_lowercase();
    let would_cite = lower.contains("\"would_cite\": true")
        || lower.contains("\"would_cite\":true")
        || (lower.contains("would cite") && !lower.contains("not cite") && !lower.contains("wouldn't"));
    (would_cite, 0.5, None)
}

fn extract_json(s: &str) -> String {
    if let (Some(start), Some(end)) = (s.find('{'), s.rfind('}')) {
        s[start..=end].to_string()
    } else {
        s.to_string()
    }
}
