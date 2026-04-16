use regex::Regex;

use crate::types::{Position, Sentiment};

pub struct ParseResult {
    pub mentioned: bool,
    pub cited: bool,
    pub position: Position,
    pub sentiment: Sentiment,
    pub snippet: Option<String>,
}

pub fn parse_response(domain: &str, response: &str) -> ParseResult {
    let response_lower = response.to_lowercase();
    let domain_lower = domain.to_lowercase();

    let domain_base = strip_tld(&domain_lower);

    let mentioned = response_lower.contains(&domain_lower)
        || response_lower.contains(domain_base);

    let cited = mentioned && {
        let escaped = regex::escape(&domain_lower);
        let link_re = Regex::new(&format!(r"https?://[^\s)]*{}", escaped)).unwrap();
        let md_re = Regex::new(&format!(r"\[[^\]]*\]\([^)]*{}[^)]*\)", escaped)).unwrap();
        link_re.is_match(response) || md_re.is_match(response)
    };

    let position = if mentioned {
        detect_position(domain_base, &response_lower)
    } else {
        Position::NotMentioned
    };

    let sentiment = if mentioned {
        detect_sentiment(domain_base, &response_lower)
    } else {
        Sentiment::Unknown
    };

    let snippet = mentioned.then(|| extract_snippet(domain_base, response)).flatten();

    ParseResult {
        mentioned,
        cited,
        position,
        sentiment,
        snippet,
    }
}

fn strip_tld(domain: &str) -> &str {
    for tld in &[".com", ".io", ".dev", ".net", ".org", ".app"] {
        if let Some(s) = domain.strip_suffix(tld) {
            return s;
        }
    }
    domain
}

fn detect_position(domain_base: &str, response_lower: &str) -> Position {
    let len = response_lower.len();
    if len == 0 {
        return Position::Middle;
    }
    let idx = match response_lower.find(domain_base) {
        Some(i) => i,
        None => return Position::NotMentioned,
    };
    let ratio = idx as f64 / len as f64;
    if ratio < 0.33 {
        Position::Top
    } else if ratio < 0.66 {
        Position::Middle
    } else {
        Position::Bottom
    }
}

fn detect_sentiment(domain_base: &str, response_lower: &str) -> Sentiment {
    let sentences: Vec<&str> = response_lower
        .split(['.', '!', '?', '\n'])
        .filter(|s| s.contains(domain_base))
        .collect();

    let context: String = if sentences.is_empty() {
        response_lower[..response_lower.len().min(500)].to_string()
    } else {
        sentences.join(" ")
    };

    const POS: &[&str] = &[
        "recommend",
        "excellent",
        "great",
        "best",
        "top",
        "popular",
        "useful",
        "powerful",
        "fast",
        "reliable",
        "easy",
        "good",
        "well",
        "favorite",
        "widely used",
        "well-known",
        "leading",
        "notable",
    ];
    const NEG: &[&str] = &[
        "avoid",
        "poor",
        "bad",
        "deprecated",
        "abandoned",
        "slow",
        "buggy",
        "complex",
        "hard",
        "difficult",
        "outdated",
        "unmaintained",
        "not recommended",
    ];

    let pos = POS.iter().filter(|w| context.contains(**w)).count();
    let neg = NEG.iter().filter(|w| context.contains(**w)).count();

    match pos.cmp(&neg) {
        std::cmp::Ordering::Greater => Sentiment::Positive,
        std::cmp::Ordering::Less => Sentiment::Negative,
        std::cmp::Ordering::Equal => Sentiment::Neutral,
    }
}

fn extract_snippet(domain_base: &str, response: &str) -> Option<String> {
    let lower = response.to_lowercase();
    let idx = lower.find(domain_base)?;
    let start = idx.saturating_sub(80);
    let end = (idx + domain_base.len() + 80).min(response.len());
    let raw = response[start..end].trim();
    Some(if raw.len() > 160 {
        format!("{}…", &raw[..160])
    } else {
        raw.to_string()
    })
}
