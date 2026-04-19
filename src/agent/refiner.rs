use std::sync::Arc;

use crate::{geo::evaluator::EvalResult, providers::LlmProvider};

pub const REFINE_SYSTEM: &str = include_str!("../../geo/templates/refine.prompt.md");

pub const GOOD_THRESHOLD: f64 = 35.0;

pub struct RefinementAttempt {
    pub content: String,
    pub model: String,
    pub score: f64,
    pub round: usize,
}

/// Build a human-readable critique from failing eval results.
pub fn build_critique(results: &[EvalResult]) -> String {
    let failing: Vec<_> = results.iter().filter(|r| !r.would_cite).collect();
    if failing.is_empty() {
        return "Evaluators did not provide specific feedback.".to_string();
    }
    failing
        .iter()
        .map(|r| {
            format!(
                "• {} ({:.0}% confidence): {}",
                r.model,
                r.confidence * 100.0,
                r.reason.as_deref().unwrap_or("would not cite — no reason given")
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Attempt one refinement pass on content that scored below the threshold.
/// Returns the improved (content, model) if a provider responded, else None.
pub async fn refine(
    prompt: &str,
    current_content: &str,
    eval_results: &[EvalResult],
    providers: &[Arc<dyn LlmProvider>],
) -> Option<(String, String)> {
    let critique = build_critique(eval_results);

    // System = rewrite rules. User message = everything the model needs to act on.
    let user_msg = format!(
        "Target query: \"{prompt}\"\n\n\
         Evaluation feedback (why the content scored low):\n{critique}\n\n\
         ---\n\nOriginal content to rewrite:\n\n{current_content}"
    );

    for provider in providers {
        match provider.query_with_system(Some(REFINE_SYSTEM), &user_msg).await {
            Ok(refined) => {
                let trimmed = refined.trim().to_string();
                if trimmed.len() > 100 {
                    return Some((trimmed, provider.name().to_string()));
                }
            }
            Err(e) => eprintln!("  [{}] refine error: {}", provider.name(), e),
        }
    }
    None
}
