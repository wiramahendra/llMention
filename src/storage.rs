use anyhow::Result;
use chrono::Utc;
use rusqlite::{params, Connection};
use std::path::PathBuf;

use crate::types::{MentionResult, Position, Sentiment};

pub struct Storage {
    conn: Connection,
}

impl Storage {
    pub fn open(base_dir: &PathBuf) -> Result<Self> {
        std::fs::create_dir_all(base_dir)?;
        let conn = Connection::open(base_dir.join("mentions.db"))?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS mentions (
                id          INTEGER PRIMARY KEY AUTOINCREMENT,
                domain      TEXT NOT NULL,
                prompt      TEXT NOT NULL,
                model       TEXT NOT NULL,
                timestamp   TEXT NOT NULL,
                mentioned   INTEGER NOT NULL,
                cited       INTEGER NOT NULL,
                position    TEXT NOT NULL,
                sentiment   TEXT NOT NULL,
                snippet     TEXT,
                raw_response TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_domain_ts ON mentions(domain, timestamp);",
        )?;
        Ok(Self { conn })
    }

    pub fn insert(&self, r: &MentionResult) -> Result<()> {
        self.conn.execute(
            "INSERT INTO mentions
             (domain,prompt,model,timestamp,mentioned,cited,position,sentiment,snippet,raw_response)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
            params![
                r.domain,
                r.prompt,
                r.model,
                r.timestamp.to_rfc3339(),
                r.mentioned as i32,
                r.cited as i32,
                r.position.to_string(),
                r.sentiment.to_string(),
                r.snippet,
                r.raw_response,
            ],
        )?;
        Ok(())
    }

    pub fn query_domain(&self, domain: &str, days: u32) -> Result<Vec<MentionResult>> {
        let since = Utc::now() - chrono::Duration::days(days as i64);
        let mut stmt = self.conn.prepare(
            "SELECT domain,prompt,model,timestamp,mentioned,cited,position,sentiment,snippet,raw_response
             FROM mentions WHERE domain=?1 AND timestamp>=?2 ORDER BY timestamp DESC",
        )?;

        let rows = stmt.query_map(params![domain, since.to_rfc3339()], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, i32>(4)? != 0,
                row.get::<_, i32>(5)? != 0,
                row.get::<_, String>(6)?,
                row.get::<_, String>(7)?,
                row.get::<_, Option<String>>(8)?,
                row.get::<_, String>(9)?,
            ))
        })?;

        let mut results = Vec::new();
        for row in rows {
            let (domain, prompt, model, ts, mentioned, cited, pos_s, sent_s, snippet, raw) = row?;
            let timestamp = chrono::DateTime::parse_from_rfc3339(&ts)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());
            results.push(MentionResult {
                domain,
                prompt,
                model,
                timestamp,
                mentioned,
                cited,
                position: parse_position(&pos_s),
                sentiment: parse_sentiment(&sent_s),
                snippet,
                raw_response: raw,
            });
        }
        Ok(results)
    }
}

fn parse_position(s: &str) -> Position {
    match s {
        "Top" => Position::Top,
        "Middle" => Position::Middle,
        "Bottom" => Position::Bottom,
        _ => Position::NotMentioned,
    }
}

fn parse_sentiment(s: &str) -> Sentiment {
    match s {
        "Positive" => Sentiment::Positive,
        "Neutral" => Sentiment::Neutral,
        "Negative" => Sentiment::Negative,
        _ => Sentiment::Unknown,
    }
}
