use colored::Colorize;
use comfy_table::{Attribute, Cell, Color, ContentArrangement, Table};

use crate::types::{MentionResult, Sentiment, TrackSummary};

pub fn print_summary(summary: &TrackSummary) {
    println!();
    println!("{}", "━".repeat(64).dimmed());
    println!(
        "  {}  {}",
        "LLMention Report:".bold(),
        summary.domain.cyan().bold()
    );
    println!("{}", "━".repeat(64).dimmed());

    let rate = summary.mention_rate();
    let rate_str = format!("{:.0}%", rate);
    let rate_colored = if rate >= 60.0 {
        rate_str.green().bold()
    } else if rate >= 30.0 {
        rate_str.yellow().bold()
    } else {
        rate_str.red().bold()
    };

    let total_models: Vec<String> = {
        let mut m: Vec<String> = summary.results.iter().map(|r| r.model.clone()).collect();
        m.sort();
        m.dedup();
        m
    };

    println!();
    println!(
        "  Mention rate   {}  ({}/{} queries)",
        rate_colored, summary.mention_count, summary.total_queries
    );
    println!("  Citations      {}", summary.citation_count);
    println!(
        "  Models active  {}/{}  ({})",
        summary.models_with_mention.len(),
        total_models.len(),
        if summary.models_with_mention.is_empty() {
            "none".red().to_string()
        } else {
            summary.models_with_mention.join(", ").cyan().to_string()
        }
    );
    println!();

    print_results_table(&summary.results);

    println!();
    print_geo_hints(summary);
    println!();
}

fn print_results_table(results: &[MentionResult]) {
    let mut table = Table::new();
    table.set_content_arrangement(ContentArrangement::Dynamic);
    table.set_header(vec![
        Cell::new("Model").add_attribute(Attribute::Bold),
        Cell::new("Prompt").add_attribute(Attribute::Bold),
        Cell::new("Mentioned").add_attribute(Attribute::Bold),
        Cell::new("Cited").add_attribute(Attribute::Bold),
        Cell::new("Position").add_attribute(Attribute::Bold),
        Cell::new("Sentiment").add_attribute(Attribute::Bold),
    ]);

    for r in results {
        let mentioned = if r.mentioned {
            Cell::new("Yes").fg(Color::Green)
        } else {
            Cell::new("No").fg(Color::Red)
        };
        let cited = if r.cited {
            Cell::new("Yes").fg(Color::Green)
        } else {
            Cell::new("—").fg(Color::DarkGrey)
        };
        table.add_row(vec![
            Cell::new(&r.model).fg(Color::Cyan),
            Cell::new(truncate(&r.prompt, 44)),
            mentioned,
            cited,
            Cell::new(r.position.to_string()),
            sentiment_cell(&r.sentiment),
        ]);
    }

    println!("{table}");
}

fn sentiment_cell(s: &Sentiment) -> Cell {
    match s {
        Sentiment::Positive => Cell::new("Positive").fg(Color::Green),
        Sentiment::Negative => Cell::new("Negative").fg(Color::Red),
        Sentiment::Neutral => Cell::new("Neutral").fg(Color::Yellow),
        Sentiment::Unknown => Cell::new("—").fg(Color::DarkGrey),
    }
}

fn print_geo_hints(summary: &TrackSummary) {
    let rate = summary.mention_rate();
    let citation_rate = if summary.total_queries == 0 {
        0.0
    } else {
        summary.citation_count as f64 / summary.total_queries as f64 * 100.0
    };

    println!("  {}", "GEO Hints:".bold());

    if rate == 0.0 {
        hint("red", "Not mentioned anywhere. Start with a dedicated product page that");
        hint("red", "  answers 'what is X', 'how does X work', and 'X vs alternative'.");
        hint("red", "  Use H2 headings that mirror exact user queries.");
    } else if rate < 30.0 {
        hint("yellow", "Low visibility. Add a concise definition paragraph at the top of");
        hint("yellow", "  your README/docs so LLMs can extract a clear entity description.");
        hint("yellow", "  Publish comparison pages (e.g. 'X vs Y') — they rank well in citations.");
    } else if rate < 60.0 {
        hint("yellow", "Moderate visibility. Lead with the direct answer in the first 2 sentences");
        hint("yellow", "  (inverted-pyramid style). Short paragraphs and bullet lists");
        hint("yellow", "  make your content easier to extract as citations.");
    } else {
        hint("green", "Strong visibility. Focus on citation quality over quantity.");
        hint("green", "  Ensure your most cited sections are factually dense and link to");
        hint("green", "  authoritative sources to reinforce trustworthiness.");
    }

    if citation_rate == 0.0 && rate > 0.0 {
        hint("yellow", "No link citations found. Publish clean, stable URLs and reference");
        hint("yellow", "  them in your own content so models learn to associate them.");
    }

    let bottom_heavy = summary
        .results
        .iter()
        .filter(|r| r.mentioned && matches!(r.position, crate::types::Position::Bottom))
        .count();
    if bottom_heavy > summary.mention_count / 2 && summary.mention_count > 0 {
        hint("yellow", "Most mentions appear at the bottom of responses. Move your brand");
        hint("yellow", "  higher by publishing content with your name in H1/H2 and in");
        hint("yellow", "  the first sentence of every major section.");
    }
}

fn hint(level: &str, msg: &str) {
    let prefix = match level {
        "green" => "  ✓".green().bold(),
        "red" => "  ✗".red().bold(),
        _ => "  →".yellow().bold(),
    };
    println!("{}  {}", prefix, msg);
}

pub fn print_trend_report(domain: &str, results: &[MentionResult], days: u32) {
    println!();
    println!("{}", "━".repeat(64).dimmed());
    println!(
        "  {}  {} — last {} days",
        "Trend Report:".bold(),
        domain.cyan().bold(),
        days
    );
    println!("{}", "━".repeat(64).dimmed());

    if results.is_empty() {
        println!(
            "\n  No data found. Run {} first.\n",
            format!("llmention track {}", domain).cyan()
        );
        return;
    }

    let total = results.len();
    let mentioned = results.iter().filter(|r| r.mentioned).count();
    let cited = results.iter().filter(|r| r.cited).count();

    println!();
    println!("  Total queries  {}", total.to_string().bold());
    println!(
        "  Mentions       {} ({:.0}%)",
        mentioned,
        mentioned as f64 / total as f64 * 100.0
    );
    println!("  Citations      {}", cited);
    println!();

    let mut models: Vec<String> = results.iter().map(|r| r.model.clone()).collect();
    models.sort();
    models.dedup();

    let mut table = Table::new();
    table.set_content_arrangement(ContentArrangement::Dynamic);
    table.set_header(vec!["Model", "Queries", "Mentions", "Rate", "Citations"]);

    for model in &models {
        let mr: Vec<&MentionResult> = results.iter().filter(|r| &r.model == model).collect();
        let q = mr.len();
        let m = mr.iter().filter(|r| r.mentioned).count();
        let c = mr.iter().filter(|r| r.cited).count();
        let rate_str = format!("{:.0}%", m as f64 / q as f64 * 100.0);
        let rate_cell = if m as f64 / q as f64 >= 0.6 {
            Cell::new(rate_str).fg(Color::Green)
        } else if m as f64 / q as f64 >= 0.3 {
            Cell::new(rate_str).fg(Color::Yellow)
        } else {
            Cell::new(rate_str).fg(Color::Red)
        };
        table.add_row(vec![
            Cell::new(model).fg(Color::Cyan),
            Cell::new(q),
            Cell::new(m),
            rate_cell,
            Cell::new(c),
        ]);
    }

    println!("{table}");
    println!();
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max - 1])
    }
}
