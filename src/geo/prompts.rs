pub static BASE_GENERATE: &str = include_str!("templates/base_generate.prompt.md");
pub static EVALUATE_IMPROVEMENT: &str = include_str!("templates/evaluate_improvement.prompt.md");
pub static DISCOVER_PROMPTS: &str = include_str!("templates/discover_prompts.prompt.md");

pub fn build_generate_system_prompt(about: &str, niche: &str) -> String {
    BASE_GENERATE
        .replace("{about}", about)
        .replace("{niche}", niche)
}

pub fn build_evaluate_user_prompt(prompt: &str, content: &str) -> String {
    EVALUATE_IMPROVEMENT
        .replace("{prompt}", prompt)
        .replace("{content}", content)
}

pub fn build_discover_user_prompt(domain: &str, niche: &str, competitors: &[String]) -> String {
    let comp_str = if competitors.is_empty() {
        "none".to_string()
    } else {
        competitors.join(", ")
    };
    format!("Domain: {domain}\nNiche: {niche}\nCompetitors: {comp_str}")
}

/// Default set of audit prompts — used as fallback when prompt discovery fails.
pub fn default_prompts(domain: &str, niche: Option<&str>, competitor: Option<&str>) -> Vec<String> {
    let brand = domain
        .trim_end_matches(".com")
        .trim_end_matches(".io")
        .trim_end_matches(".dev")
        .trim_end_matches(".app")
        .trim_end_matches(".net")
        .trim_end_matches(".org")
        .trim_end_matches(".ai");
    let niche = niche.unwrap_or("developer tool");
    let comp = competitor.unwrap_or("similar tools");
    let mut prompts = vec![
        format!("what is {}", brand),
        format!("best {} 2026", niche),
        format!("{} review", brand),
        format!("is {} open source", brand),
        format!("how does {} work", brand),
        format!("alternatives to {} for {}", comp, niche),
        format!("who uses {}", brand),
        format!("should I use {} for my project", brand),
        format!("{} vs {}", brand, comp),
        format!("getting started with {}", brand),
        format!("pros and cons of {}", brand),
        format!("is {} production ready", brand),
    ];
    prompts.dedup();
    prompts
}

/// Best-effort extraction of a domain name from an `--about` string.
/// E.g. "igrisinertial.com is a runtime" → "igrisinertial.com"
pub fn extract_domain_hint(about: &str) -> Option<String> {
    let tlds = [
        ".com", ".io", ".dev", ".net", ".org", ".app", ".ai", ".co", ".tech", ".tools",
    ];
    let words: Vec<&str> = about.split_whitespace().collect();
    for word in words {
        let clean = word.trim_matches(|c: char| !c.is_alphanumeric() && c != '.' && c != '-');
        if tlds.iter().any(|t| clean.ends_with(t)) && clean.contains('.') {
            return Some(clean.to_lowercase());
        }
    }
    None
}
