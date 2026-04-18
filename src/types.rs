use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Position {
    Top,
    Middle,
    Bottom,
    NotMentioned,
}

impl std::fmt::Display for Position {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Position::Top => write!(f, "Top"),
            Position::Middle => write!(f, "Middle"),
            Position::Bottom => write!(f, "Bottom"),
            Position::NotMentioned => write!(f, "—"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Sentiment {
    Positive,
    Neutral,
    Negative,
    Unknown,
}

impl std::fmt::Display for Sentiment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Sentiment::Positive => write!(f, "Positive"),
            Sentiment::Neutral => write!(f, "Neutral"),
            Sentiment::Negative => write!(f, "Negative"),
            Sentiment::Unknown => write!(f, "—"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MentionResult {
    pub domain: String,
    pub prompt: String,
    pub model: String,
    pub timestamp: DateTime<Utc>,
    pub mentioned: bool,
    pub cited: bool,
    pub position: Position,
    pub sentiment: Sentiment,
    pub snippet: Option<String>,
    pub raw_response: String,
}

#[derive(Debug, Clone)]
pub struct TrackSummary {
    pub domain: String,
    pub total_queries: usize,
    pub mention_count: usize,
    pub citation_count: usize,
    pub models_with_mention: Vec<String>,
    pub results: Vec<MentionResult>,
}

impl TrackSummary {
    pub fn mention_rate(&self) -> f64 {
        if self.total_queries == 0 {
            return 0.0;
        }
        self.mention_count as f64 / self.total_queries as f64 * 100.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn position_display() {
        assert_eq!(Position::Top.to_string(), "Top");
        assert_eq!(Position::Middle.to_string(), "Middle");
        assert_eq!(Position::Bottom.to_string(), "Bottom");
        assert_eq!(Position::NotMentioned.to_string(), "—");
    }

    #[test]
    fn sentiment_display() {
        assert_eq!(Sentiment::Positive.to_string(), "Positive");
        assert_eq!(Sentiment::Neutral.to_string(), "Neutral");
        assert_eq!(Sentiment::Negative.to_string(), "Negative");
        assert_eq!(Sentiment::Unknown.to_string(), "—");
    }

    #[test]
    fn mention_rate_zero_queries() {
        let s = TrackSummary {
            domain: "x.com".into(),
            total_queries: 0,
            mention_count: 0,
            citation_count: 0,
            models_with_mention: vec![],
            results: vec![],
        };
        assert_eq!(s.mention_rate(), 0.0);
    }

    #[test]
    fn mention_rate_calculation() {
        let s = TrackSummary {
            domain: "x.com".into(),
            total_queries: 4,
            mention_count: 3,
            citation_count: 1,
            models_with_mention: vec![],
            results: vec![],
        };
        assert!((s.mention_rate() - 75.0).abs() < 0.01);
    }
}
