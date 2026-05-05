use anyhow::Result;
use chrono::Utc;
use colored::Colorize;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use tokio::sync::Semaphore;

use crate::{
    audit_storage::{AuditStorage, AuditSummary, NewAuditResult, NewPrompt},
    config::Config,
    parser,
    project_config::ProjectProvidersConfig,
    providers::LlmProvider,
    types::{Position, Sentiment},
};

/// Options for running an audit
#[derive(Debug, Clone)]
pub struct AuditOptions {
    pub samples_per_prompt: usize,
    pub temperature: f32,
    pub store_raw_responses: bool,
    pub verbose: bool,
    pub quiet: bool,
    pub concurrency: usize,
}

impl Default for AuditOptions {
    fn default() -> Self {
        Self {
            samples_per_prompt: 3,
            temperature: 0.2,
            store_raw_responses: true,
            verbose: false,
            quiet: false,
            concurrency: 5,
        }
    }
}

/// A single audit query result (before storage)
#[derive(Debug, Clone)]
struct QueryResult {
    prompt_id: Option<i64>,
    provider: String,
    model: String,
    sample_index: usize,
    response_text: String,
    mentioned_project: bool,
    recommended_project: bool,
    mention_position: Position,
    sentiment: Sentiment,
    citations: Vec<(String, bool)>, // (url, is_project)
}

/// The core audit engine
pub struct AuditEngine {
    providers: Vec<Arc<dyn LlmProvider>>,
    options: AuditOptions,
}

impl AuditEngine {
    pub fn new(
        providers: Vec<Arc<dyn LlmProvider>>,
        options: AuditOptions,
    ) -> Self {
        Self {
            providers,
            options,
        }
    }

    /// Run a full audit for a project
    pub async fn run_audit(
        &self,
        project_id: &str,
        prompts: &[PromptInput],
        storage: &AuditStorage,
    ) -> Result<AuditRunResult> {
        // Create audit run record (synchronous)
        let provider_models: Vec<String> = self.providers.iter()
            .map(|p| p.name().to_string())
            .collect();
        
        let run_id = storage.create_audit_run(
            project_id,
            &provider_models,
            self.options.samples_per_prompt,
            self.options.temperature,
        )?;

        if !self.options.quiet {
            println!(
                "  {} Starting audit run {} for {} with {} prompt(s) × {} sample(s) × {} model(s)",
                "→".cyan(),
                run_id,
                project_id.cyan(),
                prompts.len(),
                self.options.samples_per_prompt,
                self.providers.len()
            );
        }

        // Store prompts first (synchronous)
        let mut stored_prompt_ids: Vec<Option<i64>> = Vec::new();
        for prompt in prompts {
            let prompt_id = if let Some(existing_id) = prompt.id {
                Some(existing_id)
            } else {
                Some(storage.insert_prompt(project_id, &NewPrompt {
                    text: &prompt.text,
                    intent: prompt.intent.as_deref(),
                    funnel_stage: prompt.funnel_stage.as_deref(),
                    priority: prompt.priority,
                    expected_entity: prompt.expected_entity.as_deref(),
                    created_by: Some("audit_engine"),
                })?)
            };
            stored_prompt_ids.push(prompt_id);
        }

        // Run all queries asynchronously and collect results
        let total_queries = prompts.len() * self.options.samples_per_prompt * self.providers.len();
        let completed = Arc::new(AtomicUsize::new(0));
        let sem = Arc::new(Semaphore::new(self.options.concurrency));

        let mut all_results: Vec<QueryResult> = Vec::new();

        for (prompt_idx, prompt) in prompts.iter().enumerate() {
            let prompt_id = stored_prompt_ids[prompt_idx];

            for (provider_idx, provider) in self.providers.iter().enumerate() {
                for sample_idx in 0..self.options.samples_per_prompt {
                    let provider = Arc::clone(provider);
                    let prompt_text = prompt.text.clone();
                    let sem = Arc::clone(&sem);
                    let completed = Arc::clone(&completed);
                    let opts = self.options.clone();
                    let project_id_owned = project_id.to_string();

                    // Execute query asynchronously
                    let result = async move {
                        let _permit = sem.acquire().await.unwrap();

                        // Query the provider
                        let response = match provider.query(&prompt_text).await {
                            Ok(resp) => resp,
                            Err(e) => {
                                eprintln!(
                                    "  {} Query failed for {}: {}",
                                    "✗".red(),
                                    provider.name().cyan(),
                                    e
                                );
                                return None;
                            }
                        };

                        // Parse the response
                        let parse_result = parser::parse_response(&project_id_owned, &response);

                        // Detect recommendation
                        let recommended = Self::detect_recommendation(&response, &project_id_owned);

                        // Extract citations
                        let citations = Self::extract_citations(&response, &project_id_owned);

                        // Progress
                        let n = completed.fetch_add(1, Ordering::SeqCst) + 1;
                        if !opts.quiet {
                            let icon = if parse_result.mentioned { "✓".green() } else { "–".dimmed() };
                            eprintln!(
                                "  {} [{:>3}/{}] [{}] sample {} — {}",
                                icon,
                                n,
                                total_queries,
                                provider.name().cyan(),
                                sample_idx + 1,
                                if parse_result.mentioned { "mentioned".green() } else { "not mentioned".dimmed() }
                            );
                            if opts.verbose {
                                eprintln!("      {}", Self::first_line(&response).dimmed());
                            }
                        }

                        Some(QueryResult {
                            prompt_id,
                            provider: provider.name().to_string(),
                            model: provider.name().to_string(),
                            sample_index: sample_idx,
                            response_text: response,
                            mentioned_project: parse_result.mentioned,
                            recommended_project: recommended,
                            mention_position: parse_result.position,
                            sentiment: parse_result.sentiment,
                            citations,
                        })
                    }.await;

                    if let Some(r) = result {
                        all_results.push(r);
                    }
                }
            }
        }

        // Store all results synchronously
        for result in &all_results {
            let raw_json = if self.options.store_raw_responses {
                serde_json::json!({
                    "provider": result.provider,
                    "prompt_id": result.prompt_id,
                    "sample_index": result.sample_index,
                    "mentioned": result.mentioned_project,
                    "recommended": result.recommended_project,
                    "timestamp": Utc::now().to_rfc3339(),
                }).to_string()
            } else {
                String::new()
            };

            let result_id = storage.insert_audit_result(&NewAuditResult {
                audit_run_id: run_id,
                prompt_id: result.prompt_id,
                provider: &result.provider,
                model: &result.model,
                sample_index: result.sample_index,
                response_text: &result.response_text,
                raw_response_json: &raw_json,
                mentioned_project: result.mentioned_project,
                recommended_project: result.recommended_project,
                mention_position: result.mention_position.clone(),
                sentiment: result.sentiment.clone(),
            })?;

            // Store citations
            for (url, is_project) in &result.citations {
                storage.insert_citation(result_id, url, *is_project)?;
            }
        }

        // Generate summary
        let summary = storage.get_audit_summary(run_id)?;
        storage.complete_audit_run(run_id, &summary)?;

        if !self.options.quiet {
            println!(
                "  {} Audit run {} completed — Mention rate: {:.1}%, Recommendation rate: {:.1}%",
                "✓".green(),
                run_id,
                summary.mention_rate * 100.0,
                summary.recommendation_rate * 100.0
            );
        }

        Ok(AuditRunResult {
            run_id,
            project_id: project_id.to_string(),
            summary,
        })
    }

    /// Detect if the response contains a recommendation
    fn detect_recommendation(response: &str, project: &str) -> bool {
        let response_lower = response.to_lowercase();
        let project_lower = project.to_lowercase();
        
        // Only check if project is mentioned
        if !response_lower.contains(&project_lower) {
            return false;
        }

        // Recommendation keywords
        const RECOMMENDATION_INDICATORS: &[&str] = &[
            "recommend",
            "best",
            "best choice",
            "good choice",
            "use",
            "try",
            "consider",
            "suggest",
            "advise",
            "optimal",
            "ideal",
            "excellent",
            "highly",
            "strongly recommend",
            "should use",
        ];

        // Find sentences mentioning the project
        let sentences: Vec<&str> = response_lower
            .split(['.', '!', '?', '\n'])
            .filter(|s| s.contains(&project_lower))
            .collect();

        for sentence in sentences {
            for indicator in RECOMMENDATION_INDICATORS {
                if sentence.contains(indicator) {
                    return true;
                }
            }
        }

        false
    }

    /// Extract URLs from response and identify if they belong to the project
    fn extract_citations(response: &str, project: &str) -> Vec<(String, bool)> {
        let url_regex = regex::Regex::new(r"https?://[^\s\)>]+").unwrap();
        let mut citations = Vec::new();

        for cap in url_regex.captures_iter(response) {
            let url = cap.get(0).unwrap().as_str().to_string();
            let domain = url.split('/').nth(2).unwrap_or("").to_lowercase();
            let project_domain = project.to_lowercase();
            
            let is_project = domain == project_domain 
                || domain.ends_with(&format!(".{}.{}.{}", 
                    project_domain.split('.').next().unwrap_or(""),
                    project_domain.split('.').nth(1).unwrap_or(""),
                    project_domain.split('.').nth(2).unwrap_or("")
                ));
            
            citations.push((url, is_project));
        }

        citations
    }

    fn first_line(s: &str) -> &str {
        s.lines().next().unwrap_or("").trim()
    }
}

/// Input prompt for auditing
#[derive(Debug, Clone)]
pub struct PromptInput {
    pub id: Option<i64>,
    pub text: String,
    pub intent: Option<String>,
    pub funnel_stage: Option<String>,
    pub priority: Option<i64>,
    pub expected_entity: Option<String>,
}

impl PromptInput {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            id: None,
            text: text.into(),
            intent: None,
            funnel_stage: None,
            priority: None,
            expected_entity: None,
        }
    }

    pub fn with_intent(mut self, intent: impl Into<String>) -> Self {
        self.intent = Some(intent.into());
        self
    }

    pub fn with_funnel_stage(mut self, stage: impl Into<String>) -> Self {
        self.funnel_stage = Some(stage.into());
        self
    }

    pub fn with_priority(mut self, priority: i64) -> Self {
        self.priority = Some(priority);
        self
    }
}

/// Result of an audit run
#[derive(Debug, Clone)]
pub struct AuditRunResult {
    pub run_id: i64,
    pub project_id: String,
    pub summary: AuditSummary,
}

/// Build providers from project config and global config
pub fn build_providers_for_project(
    project_config: &ProjectProvidersConfig,
    global_config: &Config,
    filter: Option<&str>,
) -> Vec<Arc<dyn LlmProvider>> {
    use crate::providers::{
        anthropic::AnthropicProvider,
        gemini::GeminiProvider,
        ollama::OllamaProvider,
        openai::OpenAiProvider,
        perplexity::PerplexityProvider,
        xai::XaiProvider,
    };

    let mut providers: Vec<Arc<dyn LlmProvider>> = Vec::new();

    // If specific models are requested in filter, use those
    if let Some(f) = filter {
        let names: Vec<&str> = f.split(',').map(str::trim).collect();
        for name in names {
            let parts: Vec<&str> = name.split(':').collect();
            let provider_name = parts[0];
            let model_name = parts.get(1).copied();

            match provider_name {
                "ollama" => {
                    if let Some(ref c) = global_config.providers.ollama {
                        let mut config = c.clone();
                        if let Some(m) = model_name {
                            config.model = m.to_string();
                        }
                        config.enabled = true;
                        providers.push(Arc::new(OllamaProvider::new(config)));
                    }
                }
                "openai" => {
                    if let Some(ref c) = global_config.providers.openai {
                        let mut config = c.clone();
                        if let Some(m) = model_name {
                            config.model = m.to_string();
                        }
                        config.enabled = true;
                        providers.push(Arc::new(OpenAiProvider::new(config)));
                    }
                }
                "anthropic" => {
                    if let Some(ref c) = global_config.providers.anthropic {
                        let mut config = c.clone();
                        if let Some(m) = model_name {
                            config.model = m.to_string();
                        }
                        config.enabled = true;
                        providers.push(Arc::new(AnthropicProvider::new(config)));
                    }
                }
                "xai" => {
                    if let Some(ref c) = global_config.providers.xai {
                        let mut config = c.clone();
                        if let Some(m) = model_name {
                            config.model = m.to_string();
                        }
                        config.enabled = true;
                        providers.push(Arc::new(XaiProvider::new(config)));
                    }
                }
                "gemini" => {
                    if let Some(ref c) = global_config.providers.gemini {
                        let mut config = c.clone();
                        if let Some(m) = model_name {
                            config.model = m.to_string();
                        }
                        config.enabled = true;
                        providers.push(Arc::new(GeminiProvider::new(config)));
                    }
                }
                "perplexity" => {
                    if let Some(ref c) = global_config.providers.perplexity {
                        let mut config = c.clone();
                        if let Some(m) = model_name {
                            config.model = m.to_string();
                        }
                        config.enabled = true;
                        providers.push(Arc::new(PerplexityProvider::new(config)));
                    }
                }
                _ => {}
            }
        }
        return providers;
    }

    // Otherwise, use project config or global enabled providers
    if !project_config.models.is_empty() {
        // Parse project models
        for model_str in &project_config.models {
            let parts: Vec<&str> = model_str.split(':').collect();
            if parts.len() >= 2 {
                let provider_name = parts[0];
                let model_name = parts[1];

                match provider_name {
                    "ollama" => {
                        if let Some(ref c) = global_config.providers.ollama {
                            let mut config = c.clone();
                            config.model = model_name.to_string();
                            config.enabled = true;
                            providers.push(Arc::new(OllamaProvider::new(config)));
                        }
                    }
                    "openai" => {
                        if let Some(ref c) = global_config.providers.openai {
                            let mut config = c.clone();
                            config.model = model_name.to_string();
                            config.enabled = true;
                            providers.push(Arc::new(OpenAiProvider::new(config)));
                        }
                    }
                    "anthropic" => {
                        if let Some(ref c) = global_config.providers.anthropic {
                            let mut config = c.clone();
                            config.model = model_name.to_string();
                            config.enabled = true;
                            providers.push(Arc::new(AnthropicProvider::new(config)));
                        }
                    }
                    "xai" => {
                        if let Some(ref c) = global_config.providers.xai {
                            let mut config = c.clone();
                            config.model = model_name.to_string();
                            config.enabled = true;
                            providers.push(Arc::new(XaiProvider::new(config)));
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    // Fall back to globally enabled providers
    if providers.is_empty() {
        if let Some(ref c) = global_config.providers.openai {
            if c.enabled { providers.push(Arc::new(OpenAiProvider::new(c.clone()))); }
        }
        if let Some(ref c) = global_config.providers.anthropic {
            if c.enabled { providers.push(Arc::new(AnthropicProvider::new(c.clone()))); }
        }
        if let Some(ref c) = global_config.providers.gemini {
            if c.enabled { providers.push(Arc::new(GeminiProvider::new(c.clone()))); }
        }
        if let Some(ref c) = global_config.providers.xai {
            if c.enabled { providers.push(Arc::new(XaiProvider::new(c.clone()))); }
        }
        if let Some(ref c) = global_config.providers.perplexity {
            if c.enabled { providers.push(Arc::new(PerplexityProvider::new(c.clone()))); }
        }
        if let Some(ref c) = global_config.providers.ollama {
            if c.enabled { providers.push(Arc::new(OllamaProvider::new(c.clone()))); }
        }
    }

    providers
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_recommendation() {
        let response = "I recommend using MyProject for this task.";
        assert!(AuditEngine::detect_recommendation(response, "MyProject"));

        let response2 = "MyProject is a tool that exists.";
        assert!(!AuditEngine::detect_recommendation(response2, "MyProject"));
    }

    #[test]
    fn test_extract_citations() {
        let response = "Visit https://example.com/docs and https://myproject.com for more info.";
        let citations = AuditEngine::extract_citations(response, "myproject.com");
        
        assert_eq!(citations.len(), 2);
        assert!(!citations[0].1); // example.com is not project
        assert!(citations[1].1);  // myproject.com is project
    }
}
