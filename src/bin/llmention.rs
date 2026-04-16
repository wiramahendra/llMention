use anyhow::{bail, Result};
use clap::{Parser, Subcommand};
use colored::Colorize;
use std::path::PathBuf;

use llmention::{
    cache::Cache,
    config::{Config, EXAMPLE_CONFIG},
    report,
    storage::Storage,
    tracker::{self, TrackOptions},
};

#[derive(Parser)]
#[command(
    name = "llmention",
    about = "Track how often LLMs mention your brand — local, private, fast",
    version,
    arg_required_else_help = true
)]
struct Cli {
    /// Comma-separated models to use (e.g. openai,anthropic,ollama)
    #[arg(long, short, global = true)]
    models: Option<String>,

    /// Print raw LLM responses for each query
    #[arg(long, short, global = true, default_value = "false")]
    verbose: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run prompts against configured models and record brand mentions
    Track {
        /// Domain or brand to track (e.g. myproject.com)
        domain: String,
        /// Path to prompts file (.txt one-per-line or .json array)
        #[arg(long, short)]
        prompts: Option<PathBuf>,
        /// Days window for deduplication check
        #[arg(long, short, default_value = "1")]
        days: u32,
    },
    /// Quick audit with 8–12 smart default prompts (no file needed)
    Audit {
        /// Domain or brand to audit
        domain: String,
        /// Product niche for smarter prompts (e.g. "Rust CLI tool")
        #[arg(long)]
        niche: Option<String>,
        /// Primary competitor for comparison prompts
        #[arg(long)]
        competitor: Option<String>,
    },
    /// Show mention trends from local history
    Report {
        /// Domain or brand to report on
        domain: String,
        /// Number of past days to include
        #[arg(long, short, default_value = "7")]
        days: u32,
    },
    /// Print config location and write an example config if none exists
    Config,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let config = Config::load()?;
    let base_dir = Config::ensure_dir()?;
    let storage = Storage::open(&base_dir)?;
    let cache = Cache::new(&base_dir)?;

    let opts = TrackOptions {
        verbose: cli.verbose,
        concurrency: config.defaults.concurrency,
    };

    match cli.command {
        Commands::Track {
            domain,
            prompts,
            days: _,
        } => {
            let providers = tracker::build_providers_filtered(&config, cli.models.as_deref());
            if providers.is_empty() {
                bail!(
                    "No providers enabled. Run {} to see setup instructions.",
                    "llmention config".cyan()
                );
            }
            let prompts = load_prompts(prompts, &domain)?;
            println!(
                "\n  {} {} — {} prompt(s) × {} model(s)\n",
                "Tracking".bold(),
                domain.cyan().bold(),
                prompts.len(),
                providers.len()
            );
            let summary =
                tracker::run_track(&domain, prompts, providers, &storage, &cache, opts).await?;
            report::print_summary(&summary);
        }

        Commands::Audit {
            domain,
            niche,
            competitor,
        } => {
            let providers = tracker::build_providers_filtered(&config, cli.models.as_deref());
            if providers.is_empty() {
                bail!(
                    "No providers enabled. Run {} to see setup instructions.",
                    "llmention config".cyan()
                );
            }
            let prompts = audit_prompts(&domain, niche.as_deref(), competitor.as_deref());
            println!(
                "\n  {} {} — {} prompts × {} model(s)\n",
                "Auditing".bold(),
                domain.cyan().bold(),
                prompts.len(),
                providers.len()
            );
            let summary =
                tracker::run_track(&domain, prompts, providers, &storage, &cache, opts).await?;
            report::print_summary(&summary);
        }

        Commands::Report { domain, days } => {
            let results = storage.query_domain(&domain, days)?;
            report::print_trend_report(&domain, &results, days);
        }

        Commands::Config => {
            run_config_command()?;
        }
    }

    Ok(())
}

fn run_config_command() -> Result<()> {
    let dir = Config::ensure_dir()?;
    let path = llmention::config::config_path();

    println!();
    println!("{}", "LLMention — Configuration".bold());
    println!("{}", "━".repeat(52).dimmed());
    println!();
    println!("  Config dir   {}", dir.display().to_string().cyan());
    println!("  Config file  {}", path.display().to_string().cyan());
    println!();

    if path.exists() {
        println!("  {} Config file already exists.", "✓".green());
    } else {
        std::fs::write(&path, EXAMPLE_CONFIG)?;
        println!(
            "  {} Example config written to {}",
            "✓".green(),
            path.display().to_string().cyan()
        );
        println!("  Edit it to add your API keys, then run:");
        println!("    {}", "llmention audit myproject.com".cyan());
    }

    println!();
    println!("  {}", "Supported providers:".bold());
    println!(
        "    {}  openai    — GPT-4o-mini, GPT-4o, etc.",
        "·".dimmed()
    );
    println!(
        "    {}  anthropic — claude-3-5-haiku, claude-3-5-sonnet, etc.",
        "·".dimmed()
    );
    println!(
        "    {}  xai       — grok-2-latest (x.ai)",
        "·".dimmed()
    );
    println!(
        "    {}  ollama    — any local model (llama3.2, mistral, etc.)",
        "·".dimmed()
    );
    println!();
    println!(
        "  {} Set {} for deterministic results (recommended).",
        "Tip:".yellow().bold(),
        "temperature = 0".cyan()
    );
    println!();

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
        None => Ok(audit_prompts(domain, None, None)),
    }
}

fn audit_prompts(domain: &str, niche: Option<&str>, competitor: Option<&str>) -> Vec<String> {
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
