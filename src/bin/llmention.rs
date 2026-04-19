use anyhow::{bail, Result};
use clap::{Parser, Subcommand, ValueEnum};
use colored::Colorize;
use std::path::PathBuf;

use llmention::{
    agent::optimizer::{self, OptimizeOptions},
    cache::Cache,
    config::{Config, EXAMPLE_CONFIG},
    geo::{
        evaluator,
        generator::{self, GenerateOptions},
        prompts::{self, extract_domain_hint},
    },
    report,
    storage::Storage,
    tracker::{self, TrackOptions},
};

const BANNER: &str = r#"
  ██╗     ██╗     ███╗   ███╗███████╗███╗   ██╗████████╗██╗ ██████╗ ███╗   ██╗
  ██║     ██║     ████╗ ████║██╔════╝████╗  ██║╚══██╔══╝██║██╔═══██╗████╗  ██║
  ██║     ██║     ██╔████╔██║█████╗  ██╔██╗ ██║   ██║   ██║██║   ██║██╔██╗ ██║
  ██║     ██║     ██║╚██╔╝██║██╔══╝  ██║╚██╗██║   ██║   ██║██║   ██║██║╚██╗██║
  ███████╗███████╗██║ ╚═╝ ██║███████╗██║ ╚████║   ██║   ██║╚██████╔╝██║ ╚████║
  ╚══════╝╚══════╝╚═╝     ╚═╝╚══════╝╚═╝  ╚═══╝   ╚═╝   ╚═╝ ╚═════╝ ╚═╝  ╚═══╝
"#;

#[derive(Parser)]
#[command(
    name = "llmention",
    about = "Track how often LLMs mention your brand — local, private, no SaaS",
    long_about = "LLMention queries LLM providers with prompts about your brand and\n\
                  records whether and how they mention it. All data stays on disk.\n\n\
                  Quick start:\n  \
                  llmention config                          # create config\n  \
                  llmention audit myproject.com             # quick scan\n  \
                  llmention report myproject.com --days 30  # view trends\n  \
                  llmention doctor                          # verify setup",
    version,
    arg_required_else_help = true
)]
struct Cli {
    /// Comma-separated models to query (e.g. openai,anthropic,ollama)
    #[arg(long, short, global = true)]
    models: Option<String>,

    /// Show first line of each raw LLM response
    #[arg(long, short, global = true, default_value = "false")]
    verbose: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Clone, ValueEnum)]
enum ExportFormat {
    Csv,
    Markdown,
}

#[derive(Subcommand)]
enum Commands {
    /// Run prompts against configured models and record brand mentions
    ///
    /// Examples:
    ///   llmention track myproject.com
    ///   llmention track myproject.com --prompts prompts.txt --models openai,ollama
    ///   llmention track myproject.com --judge
    Track {
        /// Domain or brand to track (e.g. myproject.com)
        domain: String,
        /// Path to prompts file (.txt one-per-line or .json array)
        #[arg(long, short)]
        prompts: Option<PathBuf>,
        /// Re-evaluate each response with a local LLM for higher-accuracy parsing
        #[arg(long)]
        judge: bool,
    },
    /// Quick audit using 12 smart default prompts — no file needed
    ///
    /// Examples:
    ///   llmention audit myproject.com
    ///   llmention audit myproject.com --niche "Rust CLI tool" --competitor ripgrep
    ///   llmention audit myproject.com --models ollama   # fully local, free
    Audit {
        /// Domain or brand to audit
        domain: String,
        /// Product niche for smarter prompt generation (e.g. "Rust CLI tool")
        #[arg(long)]
        niche: Option<String>,
        /// Main competitor for comparison prompts
        #[arg(long)]
        competitor: Option<String>,
        /// Re-evaluate each response with a local LLM for higher-accuracy parsing
        #[arg(long)]
        judge: bool,
    },
    /// Show mention history and trends from the local database
    ///
    /// Examples:
    ///   llmention report myproject.com
    ///   llmention report myproject.com --days 30
    ///   llmention report myproject.com --export csv > results.csv
    ///   llmention report myproject.com --export markdown > report.md
    Report {
        /// Domain or brand
        domain: String,
        /// Number of past days to include
        #[arg(long, short, default_value = "7")]
        days: u32,
        /// Export as structured format instead of terminal table
        #[arg(long, value_enum)]
        export: Option<ExportFormat>,
    },
    /// Autonomous GEO agent: discover prompts, audit visibility, generate content, show lift
    ///
    /// Examples:
    ///   llmention optimize igrisinertial.com --niche "deterministic edge runtime"
    ///   llmention optimize myproject.com --niche "rust cli tool" --competitors "ripgrep,fd" --steps 5
    ///   llmention optimize myproject.com --niche "..." --dry-run
    ///   llmention optimize myproject.com --niche "..." --auto-apply
    Optimize {
        /// Domain or brand to optimize (e.g. myproject.com)
        domain: String,
        /// Main niche or product category (required for relevant content)
        #[arg(long, short)]
        niche: String,
        /// Comma-separated list of competitors to benchmark against
        #[arg(long, short)]
        competitors: Option<String>,
        /// Number of weak prompts to generate content for (default: 3)
        #[arg(long, short, default_value = "3")]
        steps: usize,
        /// Show full plan and generated content without writing any files
        #[arg(long)]
        dry_run: bool,
        /// Automatically write generated sections to ./geo/ folder
        #[arg(long)]
        auto_apply: bool,
    },
    /// Generate GEO-optimized markdown content for a target query
    ///
    /// Examples:
    ///   llmention generate "best deterministic runtime for edge AI agents" --about "myproject.io is a ..."
    ///   llmention generate "alternatives to ROS 2" --niche "edge robotics" --evaluate
    ///   llmention generate "what is myproject" --about "..." --output content.md
    ///   llmention generate "..." --about "..." --models anthropic --evaluate
    Generate {
        /// Target query or topic to generate content for
        prompt: String,
        /// Short description of your project (e.g. "myproject.io is a fast Rust CLI for GEO")
        #[arg(long, short)]
        about: Option<String>,
        /// Optional niche/context for more targeted content (e.g. "edge robotics", "Rust CLI tools")
        #[arg(long, short)]
        niche: Option<String>,
        /// Save generated content to a file instead of printing to stdout
        #[arg(long, short)]
        output: Option<PathBuf>,
        /// After generating, estimate before/after visibility lift with LLM evaluation
        #[arg(long, short)]
        evaluate: bool,
    },
    /// Create config file and show setup instructions
    Config,
    /// Check your setup: config, providers, Ollama connectivity
    Doctor,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let config = Config::load()?;
    let (base_dir, is_first_run) = Config::ensure_dir()?;

    if is_first_run {
        print_welcome();
        // Bootstrap config on very first run
        let path = llmention::config::config_path();
        if !path.exists() {
            std::fs::write(&path, EXAMPLE_CONFIG)?;
            println!(
                "  {} Config created at {} — edit it with your API keys,",
                "✅".green(),
                path.display().to_string().cyan()
            );
            println!("     or set {} to use Ollama for free.\n", "enabled = true".cyan());
        }
    }

    let storage = Storage::open(&base_dir)?;
    let cache = Cache::new(&base_dir)?;

    match cli.command {
        Commands::Track { domain, prompts, judge } => {
            let providers = tracker::build_providers_filtered(&config, cli.models.as_deref());
            if providers.is_empty() {
                no_providers_error();
            }
            let prompts = load_prompts(prompts, &domain)?;
            if prompts.is_empty() {
                bail!("Prompts file is empty. Add at least one prompt (one per line).");
            }
            let judge_provider = build_judge_provider(judge, &config);

            println!(
                "\n  {} {} — {} prompt(s) × {} model(s){}\n",
                "Tracking".bold(),
                domain.cyan().bold(),
                prompts.len(),
                providers.len(),
                if judge { "  [+judge]".dimmed().to_string() } else { String::new() }
            );

            let prev_rate = fetch_prev_rate(&storage, &domain);
            let summary = tracker::run_track(
                &domain, prompts, providers, &storage, &cache,
                TrackOptions {
                    verbose: cli.verbose,
                    concurrency: config.defaults.concurrency,
                    judge: judge_provider,
                },
            ).await?;
            report::print_summary(&summary, prev_rate);
        }

        Commands::Audit { domain, niche, competitor, judge } => {
            let providers = tracker::build_providers_filtered(&config, cli.models.as_deref());
            if providers.is_empty() {
                no_providers_error();
            }
            let prompts = audit_prompts(&domain, niche.as_deref(), competitor.as_deref());
            let judge_provider = build_judge_provider(judge, &config);

            println!(
                "\n  {} {} — {} prompts × {} model(s){}\n",
                "Auditing".bold(),
                domain.cyan().bold(),
                prompts.len(),
                providers.len(),
                if judge { "  [+judge]".dimmed().to_string() } else { String::new() }
            );

            let prev_rate = fetch_prev_rate(&storage, &domain);
            let summary = tracker::run_track(
                &domain, prompts, providers, &storage, &cache,
                TrackOptions {
                    verbose: cli.verbose,
                    concurrency: config.defaults.concurrency,
                    judge: judge_provider,
                },
            ).await?;
            report::print_summary(&summary, prev_rate);
        }

        Commands::Report { domain, days, export } => {
            let results = storage.query_domain(&domain, days)?;
            match export {
                Some(ExportFormat::Csv) => print!("{}", report::export_csv(&results)),
                Some(ExportFormat::Markdown) => {
                    print!("{}", report::export_markdown(&results, &domain))
                }
                None => report::print_trend_report(&domain, &results, days),
            }
        }

        Commands::Optimize { domain, niche, competitors, steps, dry_run, auto_apply } => {
            let providers = tracker::build_providers_filtered(&config, cli.models.as_deref());
            if providers.is_empty() {
                no_providers_error();
            }

            let competitors_list: Vec<String> = competitors
                .as_deref()
                .unwrap_or("")
                .split(',')
                .filter(|s| !s.trim().is_empty())
                .map(|s| s.trim().to_string())
                .collect();

            println!(
                "\n  {}  {}\n  {}  {}\n  {}  {} steps{}\n",
                "Optimizing".bold(),
                domain.cyan().bold(),
                "Niche:".dimmed(),
                niche.cyan(),
                "Mode:".dimmed(),
                steps,
                if dry_run { "  [dry-run]".yellow().to_string() } else { String::new() }
            );

            let opts = OptimizeOptions {
                domain: domain.clone(),
                niche,
                competitors: competitors_list,
                steps,
                dry_run,
                verbose: cli.verbose,
            };

            let plan = optimizer::optimize(&opts, &providers, &storage, &cache).await?;

            report::print_optimization_plan(&plan, dry_run);

            if !dry_run && auto_apply && !plan.sections.is_empty() {
                std::fs::create_dir_all("geo")?;
                let mut written = 0usize;
                for section in &plan.sections {
                    let path = std::path::Path::new(&section.file_name);
                    if let Some(parent) = path.parent() {
                        std::fs::create_dir_all(parent)?;
                    }
                    std::fs::write(path, &section.content)?;
                    println!(
                        "  {}  {}",
                        "✓".green(),
                        section.file_name.cyan()
                    );
                    written += 1;
                }
                println!(
                    "\n  {} {} file(s) written to {}",
                    "✓".green().bold(),
                    written,
                    "./geo/".cyan()
                );
                println!(
                    "\n  {}  git add geo/ && git commit -m \"docs: add GEO-optimized content\"\n",
                    "→".cyan()
                );
            } else if !dry_run && !auto_apply && !plan.sections.is_empty() {
                println!(
                    "  {}  Run with {} to write {} file(s) to {}\n",
                    "Tip".yellow().bold(),
                    "--auto-apply".cyan(),
                    plan.sections.len(),
                    "./geo/".cyan()
                );
            }
        }

        Commands::Generate { prompt, about, niche, output, evaluate } => {
            let providers = tracker::build_providers_filtered(&config, cli.models.as_deref());
            if providers.is_empty() {
                no_providers_error();
            }

            let about_str = about.as_deref().unwrap_or("").to_string();
            let niche_str = niche.as_deref().unwrap_or("general").to_string();

            println!(
                "\n  {} {}\n  {} {}\n",
                "Generating content for:".bold(),
                format!("\"{}\"", prompt).cyan(),
                "Using models:".dimmed(),
                providers.iter().map(|p| p.name()).collect::<Vec<_>>().join(", ").cyan()
            );

            let opts = GenerateOptions {
                prompt: prompt.clone(),
                about: about_str.clone(),
                niche: niche_str,
                verbose: cli.verbose,
            };

            let results = generator::generate(&opts, &providers).await?;

            if results.is_empty() {
                bail!("No providers returned a response. Check your config or try --models.");
            }

            match &output {
                Some(path) => {
                    let primary = &results[0];
                    std::fs::write(path, &primary.content)?;
                    println!(
                        "  {} Saved to {}\n",
                        "✓".green().bold(),
                        path.display().to_string().cyan()
                    );
                    println!(
                        "  {}  git add {} && git commit -m \"docs: add GEO content for '{}'\"",
                        "Tip".yellow().bold(),
                        path.display(),
                        prompt
                    );
                    println!();
                }
                None => {
                    report::print_generate_results(&results, &prompt);
                }
            }

            if evaluate {
                println!("  {} Running before/after evaluation…\n", "→".cyan());

                let domain_hint = extract_domain_hint(&about_str);
                let before_stored = domain_hint.as_deref().and_then(|d| {
                    storage.query_domain(d, 7).ok().and_then(|results| {
                        if results.is_empty() {
                            None
                        } else {
                            let mentioned = results.iter().filter(|r| r.mentioned).count();
                            Some(mentioned as f64 / results.len() as f64 * 100.0)
                        }
                    })
                });

                let primary_content = &results[0].content;
                let delta = evaluator::evaluate_content(&prompt, primary_content, &providers).await?;
                report::print_eval_delta(&delta, before_stored);
            }
        }

        Commands::Config => run_config_command()?,

        Commands::Doctor => run_doctor(&config, &base_dir).await?,
    }

    Ok(())
}

// ── helpers ──────────────────────────────────────────────────────────────────

fn print_welcome() {
    println!("{}", BANNER.cyan().dimmed());
    println!("{}", "━".repeat(62).dimmed());
    println!("  Track how often LLMs mention your brand — privately,");
    println!("  locally, and without paying for a SaaS dashboard.");
    println!("{}", "━".repeat(62).dimmed());
    println!();
}

fn no_providers_error() -> ! {
    eprintln!(
        "\n  {} No providers are enabled.\n",
        "Error:".red().bold()
    );
    eprintln!("  Options:");
    eprintln!(
        "    • Add an API key in {}",
        "~/.llmention/config.toml".cyan()
    );
    eprintln!(
        "    • Or run {} and set {} for free local inference",
        "ollama serve".cyan(),
        "enabled = true".cyan()
    );
    eprintln!(
        "\n  Run {} to see setup instructions.\n",
        "llmention config".cyan()
    );
    std::process::exit(1);
}

fn run_config_command() -> Result<()> {
    let (dir, _) = Config::ensure_dir()?;
    let path = llmention::config::config_path();

    println!();
    println!("{}", "LLMention — Configuration".bold());
    println!("{}", "━".repeat(56).dimmed());
    println!();
    println!("  Config dir   {}", dir.display().to_string().cyan());
    println!("  Config file  {}", path.display().to_string().cyan());
    println!();

    if path.exists() {
        println!(
            "  {} Config already exists — edit it to add or update keys.",
            "✓".green()
        );
    } else {
        std::fs::write(&path, EXAMPLE_CONFIG)?;
        println!(
            "  {} Created {}",
            "✅".green(),
            path.display().to_string().cyan()
        );
        println!("     Edit it with your API keys, or set");
        println!(
            "     {} under [providers.ollama] for zero-cost local inference.",
            "enabled = true".cyan()
        );
    }

    println!();
    println!("  {}", "Supported providers:".bold());
    println!("    {}  openai     — gpt-4o-mini, gpt-4o, …", "·".dimmed());
    println!("    {}  anthropic  — claude-3-5-haiku, claude-3-5-sonnet, …", "·".dimmed());
    println!("    {}  xai        — grok-2-latest  (x.ai)", "·".dimmed());
    println!("    {}  perplexity — sonar, sonar-pro  (web-search grounded)", "·".dimmed());
    println!(
        "    {}  ollama     — llama3.2, mistral, phi4, …  (local, free)",
        "·".dimmed()
    );
    println!();
    println!(
        "  {}  Set {} for deterministic, cacheable results.",
        "Tip".yellow().bold(),
        "temperature = 0".cyan()
    );
    println!(
        "  {}  Run {} after editing to verify your setup.",
        "Tip".yellow().bold(),
        "llmention doctor".cyan()
    );
    println!();
    Ok(())
}

async fn run_doctor(config: &Config, base_dir: &PathBuf) -> Result<()> {
    println!();
    println!("{}", "LLMention Doctor".bold());
    println!("{}", "━".repeat(56).dimmed());
    println!();

    // ── Paths ──
    let config_path = llmention::config::config_path();
    check("Config file  ", config_path.exists(), &config_path.display().to_string());
    check("Cache dir    ", base_dir.join("cache").exists(), "~/.llmention/cache/");
    check("Database     ", base_dir.join("mentions.db").exists(), "~/.llmention/mentions.db");

    println!();
    println!("  {}", "Providers:".bold());

    let mut any_enabled = false;

    // OpenAI
    match &config.providers.openai {
        Some(c) if c.enabled => {
            any_enabled = true;
            println!("  {}  openai    {} ({})", "✓".green(), "enabled".green(), c.model.dimmed());
        }
        Some(_) => println!("  {}  openai    disabled", "–".dimmed()),
        None => println!("  {}  openai    not configured", "–".dimmed()),
    }

    // Anthropic
    match &config.providers.anthropic {
        Some(c) if c.enabled => {
            any_enabled = true;
            println!("  {}  anthropic {} ({})", "✓".green(), "enabled".green(), c.model.dimmed());
        }
        Some(_) => println!("  {}  anthropic disabled", "–".dimmed()),
        None => println!("  {}  anthropic not configured", "–".dimmed()),
    }

    // Perplexity
    match &config.providers.perplexity {
        Some(c) if c.enabled => {
            any_enabled = true;
            println!("  {}  perplexity {} ({})", "✓".green(), "enabled".green(), c.model.dimmed());
        }
        Some(_) => println!("  {}  perplexity disabled", "–".dimmed()),
        None => println!("  {}  perplexity not configured", "–".dimmed()),
    }

    // xAI
    match &config.providers.xai {
        Some(c) if c.enabled => {
            any_enabled = true;
            println!("  {}  xai       {} ({})", "✓".green(), "enabled".green(), c.model.dimmed());
        }
        Some(_) => println!("  {}  xai       disabled", "–".dimmed()),
        None => println!("  {}  xai       not configured", "–".dimmed()),
    }

    // Ollama — do a live connectivity check
    match &config.providers.ollama {
        Some(c) if c.enabled => {
            any_enabled = true;
            let reachable = ping_ollama(&c.base_url).await;
            if reachable {
                println!(
                    "  {}  ollama    {} ({}, {})",
                    "✓".green(),
                    "enabled".green(),
                    c.model.dimmed(),
                    "reachable".green()
                );
            } else {
                println!(
                    "  {}  ollama    {} — {} is not responding",
                    "!".yellow().bold(),
                    "enabled but unreachable".yellow(),
                    c.base_url.cyan()
                );
                println!(
                    "       Start it with: {}",
                    "ollama serve".cyan()
                );
            }
        }
        Some(c) => {
            // Check reachability even when disabled, to help user enable it
            let reachable = ping_ollama(&c.base_url).await;
            if reachable {
                println!(
                    "  {}  ollama    disabled (but {} — set {} to use it)",
                    "–".dimmed(),
                    "running".green(),
                    "enabled = true".cyan()
                );
            } else {
                println!("  {}  ollama    disabled", "–".dimmed());
            }
        }
        None => println!("  {}  ollama    not configured", "–".dimmed()),
    }

    println!();
    if any_enabled {
        println!(
            "  {} At least one provider is active. Try: {}",
            "✓".green().bold(),
            "llmention audit myproject.com".cyan()
        );
    } else {
        println!(
            "  {} No providers enabled. Edit {} to get started.",
            "✗".red().bold(),
            "~/.llmention/config.toml".cyan()
        );
    }
    println!();
    Ok(())
}

async fn ping_ollama(base_url: &str) -> bool {
    let url = format!("{}/api/tags", base_url);
    reqwest::Client::new()
        .get(&url)
        .timeout(std::time::Duration::from_secs(3))
        .send()
        .await
        .map(|r| r.status().is_success())
        .unwrap_or(false)
}

fn check(label: &str, ok: bool, detail: &str) {
    if ok {
        println!("  {}  {}  {}", "✓".green(), label, detail.dimmed());
    } else {
        println!("  {}  {}  {} (missing)", "✗".red(), label, detail.dimmed());
    }
}

fn build_judge_provider(
    flag: bool,
    config: &Config,
) -> Option<std::sync::Arc<dyn llmention::providers::LlmProvider>> {
    if flag || config.judge.enabled {
        tracker::build_judge(config)
    } else {
        None
    }
}

fn fetch_prev_rate(storage: &Storage, domain: &str) -> Option<f64> {
    let before = chrono::Utc::now().to_rfc3339();
    match storage.previous_run_stats(domain, &before) {
        Ok(Some((m, t))) if t > 0 => Some(m as f64 / t as f64 * 100.0),
        _ => None,
    }
}

fn load_prompts(path: Option<PathBuf>, domain: &str) -> Result<Vec<String>> {
    match path {
        Some(p) => {
            let contents = std::fs::read_to_string(&p)
                .map_err(|e| anyhow::anyhow!("Cannot read prompts file {}: {}", p.display(), e))?;
            if p.extension().map_or(false, |e| e == "json") {
                serde_json::from_str(&contents)
                    .map_err(|e| anyhow::anyhow!("Invalid JSON in {}: {}", p.display(), e))
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
    prompts::default_prompts(domain, niche, competitor)
}
