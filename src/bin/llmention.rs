use anyhow::{bail, Result};
use clap::{Parser, Subcommand};
use colored::Colorize;
use std::path::PathBuf;

use llmention::{cache::Cache, config::Config, report, storage::Storage, tracker};

#[derive(Parser)]
#[command(
    name = "llmention",
    about = "Track how often LLMs mention your brand or domain",
    version,
    arg_required_else_help = true
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run prompts against configured models and track brand mentions
    Track {
        /// Domain or brand to track (e.g. myproject.com)
        domain: String,
        /// Path to prompts file (.txt one-per-line or .json array)
        #[arg(long, short)]
        prompts: Option<PathBuf>,
        /// Comma-separated models to use (e.g. openai,anthropic,ollama)
        #[arg(long, short)]
        models: Option<String>,
        /// Days window used when deduplicating stored results
        #[arg(long, short, default_value = "1")]
        days: u32,
    },
    /// Quick audit with smart default prompts (no flags required)
    Audit {
        /// Domain or brand to audit
        domain: String,
        /// Product niche for smarter prompt generation
        #[arg(long)]
        niche: Option<String>,
        /// Comma-separated models to use
        #[arg(long, short)]
        models: Option<String>,
    },
    /// Show mention trends from local history
    Report {
        /// Domain or brand to report on
        domain: String,
        /// Number of past days to include
        #[arg(long, short, default_value = "7")]
        days: u32,
    },
    /// Show config file location and example setup
    Config,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let config = Config::load()?;
    let base_dir = Config::config_dir();
    let storage = Storage::open(&base_dir)?;
    let cache = Cache::new(&base_dir)?;

    match cli.command {
        Commands::Track {
            domain,
            prompts,
            models,
            days: _,
        } => {
            let providers = tracker::build_providers_filtered(&config, models.as_deref());
            if providers.is_empty() {
                bail!(
                    "No providers enabled. Run {} to see setup instructions.",
                    "llmention config".cyan()
                );
            }
            let prompts = load_prompts(prompts, &domain)?;
            println!(
                "\n  {} {} across {} model(s)…\n",
                "Tracking".bold(),
                domain.cyan().bold(),
                providers.len()
            );
            let summary =
                tracker::run_track(&domain, prompts, providers, &storage, &cache).await?;
            report::print_summary(&summary);
        }

        Commands::Audit {
            domain,
            niche,
            models,
        } => {
            let providers = tracker::build_providers_filtered(&config, models.as_deref());
            if providers.is_empty() {
                bail!(
                    "No providers enabled. Run {} to see setup instructions.",
                    "llmention config".cyan()
                );
            }
            let prompts = default_audit_prompts(&domain, niche.as_deref());
            println!(
                "\n  {} {} with {} prompts across {} model(s)…\n",
                "Auditing".bold(),
                domain.cyan().bold(),
                prompts.len(),
                providers.len()
            );
            let summary =
                tracker::run_track(&domain, prompts, providers, &storage, &cache).await?;
            report::print_summary(&summary);
        }

        Commands::Report { domain, days } => {
            let results = storage.query_domain(&domain, days)?;
            report::print_trend_report(&domain, &results, days);
        }

        Commands::Config => {
            print_config_help(&config);
        }
    }

    Ok(())
}

fn load_prompts(path: Option<PathBuf>, domain: &str) -> Result<Vec<String>> {
    match path {
        Some(p) => {
            let contents = std::fs::read_to_string(&p)?;
            if p.extension().map_or(false, |e| e == "json") {
                Ok(serde_json::from_str(&contents)?)
            } else {
                Ok(contents
                    .lines()
                    .filter(|l| !l.trim().is_empty())
                    .map(String::from)
                    .collect())
            }
        }
        None => Ok(default_audit_prompts(domain, None)),
    }
}

fn default_audit_prompts(domain: &str, niche: Option<&str>) -> Vec<String> {
    let brand = domain
        .trim_end_matches(".com")
        .trim_end_matches(".io")
        .trim_end_matches(".dev")
        .trim_end_matches(".app")
        .trim_end_matches(".net")
        .trim_end_matches(".org");
    let niche = niche.unwrap_or("developer tool");
    vec![
        format!("what is {}", brand),
        format!("best {} 2026", niche),
        format!("{} review", brand),
        format!("is {} open source", brand),
        format!("how does {} work", brand),
        format!("alternatives to {} for {}", brand, niche),
        format!("who uses {}", brand),
        format!("should I use {} for my project", brand),
    ]
}

fn print_config_help(config: &Config) {
    let path = llmention::config::config_path();
    println!();
    println!("{}", "LLMention — Configuration".bold());
    println!("{}", "━".repeat(50).dimmed());
    println!();
    println!("  Config file  {}", path.display().to_string().cyan());
    println!();
    println!("  {}", "Example ~/.llmention/config.toml:".bold());
    println!(
        r#"
  [providers.openai]
  api_key   = "sk-..."
  model     = "gpt-4o-mini"
  enabled   = true

  [providers.anthropic]
  api_key   = "sk-ant-..."
  model     = "claude-3-5-haiku-20241022"
  enabled   = true

  [providers.xai]
  api_key   = "xai-..."
  model     = "grok-2-latest"
  enabled   = false

  [providers.ollama]
  base_url  = "http://localhost:11434"
  model     = "llama3.2"
  enabled   = false
"#
    );

    let any_configured = config.providers.openai.is_some()
        || config.providers.anthropic.is_some()
        || config.providers.xai.is_some()
        || config.providers.ollama.is_some();

    if !any_configured {
        println!(
            "  {}  No providers configured. Create {} to get started.\n",
            "→".yellow(),
            path.display().to_string().cyan()
        );
    }
}
