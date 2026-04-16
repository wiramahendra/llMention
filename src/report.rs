use colored::Colorize;
use comfy_table::{Attribute, Cell, Color, ContentArrangement, Table};

use crate::types::{MentionResult, TrackSummary};

pub fn print_summary(summary: &TrackSummary) {
    println!();
    println!("{}", "━".repeat(62).dimmed());
    println!(
        "  {}  {}",
        "LLMention Report:".bold(),
        summary.domain.cyan().bold()
    );
    println!("{}", "━".repeat(62).dimmed());

    let rate = summary.mention_rate();
    let rate_str = format!("{:.0}%", rate);
    let rate_colored = if rate >= 60.0 {
        rate_str.green().bold()
    } else if rate >= 30.0 {
        rate_str.yellow().bold()
    } else {
        rate_str.red().bold()
    };

    println!();
    println!(
        "  Mention rate   {}  ({} / {} queries)",
        rate_colored, summary.mention_count, summary.total_queries
    );
    println!("  Citations      {}", summary.citation_count);
    if !summary.models_with_mention.is_empty() {
        println!(
            "  Models         {}",
            summary.models_with_mention.join(", ").cyan()
        );
    }
    println!();

    print_results_table(&summary.results);

    println!();
    if rate < 30.0 {
        println!(
            "  {}  Low visibility. Add structured FAQ sections and entity definitions",
            "Hint".yellow().bold()
        );
        println!("        optimised for LLM citation patterns.");
    } else if rate < 60.0 {
        println!(
            "  {}  Moderate visibility. Publish comparison pages and use-case guides.",
            "Hint".yellow().bold()
        );
    } else {
        println!(
            "  {}  Strong visibility. Focus on citation quality and authoritative links.",
            "Hint".green().bold()
        );
    }
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
        let prompt_short = truncate(&r.prompt, 42);
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
            Cell::new(prompt_short),
            mentioned,
            cited,
            Cell::new(r.position.to_string()),
            Cell::new(r.sentiment.to_string()),
        ]);
    }

    println!("{table}");
}

pub fn print_trend_report(domain: &str, results: &[MentionResult], days: u32) {
    println!();
    println!("{}", "━".repeat(62).dimmed());
    println!(
        "  {}  {} — last {} days",
        "Trend Report:".bold(),
        domain.cyan().bold(),
        days
    );
    println!("{}", "━".repeat(62).dimmed());

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
    println!(
        "  Total queries  {}",
        total.to_string().bold()
    );
    println!(
        "  Mentions       {} ({:.0}%)",
        mentioned,
        mentioned as f64 / total as f64 * 100.0
    );
    println!("  Citations      {}", cited);
    println!();
    println!("  {}", "Per-model breakdown:".bold());

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
        table.add_row(vec![
            Cell::new(model).fg(Color::Cyan),
            Cell::new(q),
            Cell::new(m),
            Cell::new(format!("{:.0}%", m as f64 / q as f64 * 100.0)),
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
