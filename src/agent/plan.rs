pub struct GeneratedSection {
    pub prompt: String,
    pub content: String,
    pub model: String,
    /// Average citability score (0–100) across all evaluating providers.
    pub citability_rate: f64,
    /// Suggested filename, e.g. "geo/best-deterministic-runtime.md"
    pub file_name: String,
}

pub struct OptimizationPlan {
    pub domain: String,
    pub niche: String,
    /// Current mention rate from the fresh audit (0–100).
    pub current_mention_rate: f64,
    pub total_audit_queries: usize,
    pub discovered_prompts: Vec<String>,
    /// The weak prompts selected for content generation.
    pub weak_prompts: Vec<String>,
    pub sections: Vec<GeneratedSection>,
}

impl OptimizationPlan {
    /// Estimated average citability of the generated sections.
    pub fn avg_citability(&self) -> f64 {
        if self.sections.is_empty() {
            return 0.0;
        }
        self.sections.iter().map(|s| s.citability_rate).sum::<f64>() / self.sections.len() as f64
    }

    /// Projected lift: avg citability of new sections minus current mention rate on those topics.
    pub fn projected_lift(&self) -> f64 {
        (self.avg_citability() - self.current_mention_rate).max(0.0)
    }
}

/// Convert a prompt string into a safe kebab-case filename (max 52 chars + ".md").
pub fn prompt_to_filename(prompt: &str) -> String {
    let slug: String = prompt
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>();

    let slug = slug
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-");

    let max = 52;
    let truncated = if slug.len() > max {
        slug[..max].trim_end_matches('-').to_string()
    } else {
        slug
    };

    format!("geo/{}.md", truncated)
}
