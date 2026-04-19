use std::sync::Arc;

use crate::{
    geo::prompts::{self, DISCOVER_PROMPTS},
    providers::LlmProvider,
};

/// Query the first responsive provider for high-intent prompts about the domain/niche.
/// Falls back to default_prompts() on parse failure or empty result.
pub async fn discover_high_intent_prompts(
    domain: &str,
    niche: &str,
    competitors: &[String],
) -> Vec<String> {
    // Return placeholder — actual discovery requires providers passed in
    prompts::default_prompts(domain, Some(niche), competitors.first().map(String::as_str))
}

/// Query providers to discover prompts, trying each until one returns a valid list.
/// `system_override` replaces the default discover system prompt (for plugin templates).
pub async fn discover_with_providers(
    domain: &str,
    niche: &str,
    competitors: &[String],
    providers: &[Arc<dyn LlmProvider>],
    system_override: Option<&str>,
) -> Vec<String> {
    let user_prompt = prompts::build_discover_user_prompt(domain, niche, competitors);
    let system = system_override.unwrap_or(DISCOVER_PROMPTS);

    for provider in providers {
        match provider.query_with_system(Some(system), &user_prompt).await {
            Ok(response) => {
                let parsed = parse_prompt_list(&response);
                if parsed.len() >= 5 {
                    return parsed;
                }
                eprintln!(
                    "  [{}] discovery returned {} prompts — trying next provider",
                    provider.name(),
                    parsed.len()
                );
            }
            Err(e) => {
                eprintln!("  [{}] discovery error: {}", provider.name(), e);
            }
        }
    }

    // Graceful fallback
    eprintln!("  → Using default prompt set (discovery did not return enough prompts)");
    prompts::default_prompts(domain, Some(niche), competitors.first().map(String::as_str))
}

fn parse_prompt_list(response: &str) -> Vec<String> {
    // Strip markdown code fences if present
    let s = response.trim();
    let s = s
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    // Find JSON array boundaries
    if let (Some(start), Some(end)) = (s.find('['), s.rfind(']')) {
        let json = &s[start..=end];
        if let Ok(list) = serde_json::from_str::<Vec<String>>(json) {
            return list
                .into_iter()
                .filter(|p| p.len() >= 5)
                .collect();
        }
    }

    // Fallback: extract quoted strings line by line
    response
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            // Match lines that look like "prompt text" or - prompt text
            let stripped = line
                .trim_start_matches('-')
                .trim_start_matches('*')
                .trim_start_matches(|c: char| c.is_ascii_digit() || c == '.')
                .trim()
                .trim_matches('"')
                .trim_matches(',')
                .trim();
            if stripped.len() >= 8 && !stripped.starts_with('{') && !stripped.starts_with('[') {
                Some(stripped.to_string())
            } else {
                None
            }
        })
        .collect()
}
