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
    marketplace::{builtin, registry},
    plugins,
    report,
    storage::Storage,
    tracker::{self, TrackOptions},
    tui,
};

const BANNER: &str = r#"
   ██╗     ██╗     ███╗   ███╗███████╗███╗   ██╗████████╗██╗ ██████╗ ███╗   ██╗
   ██║     ██║     ████╗ ████║██╔════╝████╗  ██║╚══██╔══╝██║██╔═══██╗████╗  ██║
   ██║     ██║     ██╔████╔██║█████╗  ██╔██╗ ██║   ██║   ██║██║   ██║██╔██╗ ██║
   ██║     ██║     ██║╚██╔╝██║██╔══╝  ██║╚██╗██║   ██║   ██║██║   ██║██║╚██╗██║
   ███████╗███████╗██║ ╚═╝ ██║███████╗██║ ╚████║   ██║   ██║╚██████╔╝██║ ╚████║
   ╚══════╝╚══════╝╚═╝     ╚═╝╚══════╝╚═╝  ╚═══╝   ╚═╝   ╚═╝ ╚═════╝ ╚═╝  ╚═══╝
"#;

const TAGLINE: &str = "The private, local-first GEO companion for indie builders — track, generate, and optimize your visibility in AI answers.";

#[derive(Parser)]
#[command(
    name = "llmention",
    about = "The private, local-first GEO companion for indie builders",
    long_about = "LLMention — The private, local-first GEO companion for indie builders.

Track, generate, and optimize your visibility in AI answers (ChatGPT, Claude, Perplexity, Grok, Ollama).

Quick start:
  llmention config                          # create config
  llmention audit myproject.com             # quick scan
  llmention optimize myproject.com --niche \"your niche\"  # auto-optimize
  llmention quickstart                      # guided beginner flow

Key commands:
  audit    — Quick visibility scan (12 smart prompts)
  optimize — Autonomous 5-step GEO agent
  generate — Create LLM-citable content
  report   — View trends over time",
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

    /// Suppress progress output — print only the final result (for CI/scripts)
    #[arg(long, short, global = true, default_value = "false")]
    quiet: bool,

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
    ///   llmention optimize myproject.com --niche "Rust CLI" --plugin rust-crate
    ///   llmention optimize myproject.com --niche "..." --max-rounds 3
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
        /// Max refinement rounds per section when citability is low (default: 3)
        #[arg(long, default_value = "3")]
        max_rounds: usize,
        /// Show full plan and generated content without writing any files
        #[arg(long)]
        dry_run: bool,
        /// Automatically write generated sections to ./geo/ folder
        #[arg(long)]
        auto_apply: bool,
        /// Apply a named plugin template (installed or builtin)
        #[arg(long)]
        plugin: Option<String>,
    },
    /// Generate GEO-optimized markdown content for a target query
    ///
    /// Examples:
    ///   llmention generate "best deterministic runtime for edge AI agents" --about "myproject.io is a ..."
    ///   llmention generate "alternatives to ROS 2" --niche "edge robotics" --evaluate
    ///   llmention generate "what is myproject" --about "..." --output content.md
    ///   llmention generate "..." --about "..." --models anthropic --evaluate
    ///   llmention generate "best rust cli tool" --plugin rust-crate --about "..."
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
        /// Apply a named plugin template (installed or builtin)
        #[arg(long)]
        plugin: Option<String>,
    },
    /// Manage saved projects (domain + niche pairs for quick re-auditing)
    ///
    /// Examples:
    ///   llmention projects
    ///   llmention projects add myproject.com --niche "Rust CLI tool"
    ///   llmention projects remove myproject.com
    Projects {
        #[command(subcommand)]
        action: Option<ProjectAction>,
    },
    /// Watch a domain and re-audit it on a fixed interval
    ///
    /// Examples:
    ///   llmention watch myproject.com --niche "Rust CLI tool"
    ///   llmention watch myproject.com --interval 30 --models ollama
    Watch {
        /// Domain or brand to watch
        domain: String,
        /// Product niche for smarter prompts
        #[arg(long)]
        niche: Option<String>,
        /// Interval in minutes between audits (default: 60)
        #[arg(long, short, default_value = "60")]
        interval: u64,
    },
    /// Manage installed prompt plugins
    ///
    /// Examples:
    ///   llmention plugins
    ///   llmention plugins enable rust-crate
    ///   llmention plugins disable rust-crate
    Plugins {
        #[command(subcommand)]
        action: Option<PluginAction>,
    },
    /// Browse and install community prompt templates
    ///
    /// Examples:
    ///   llmention prompts list
    ///   llmention prompts search rust
    ///   llmention prompts install rust-crate
    Prompts {
        #[command(subcommand)]
        action: PromptMarketAction,
    },
    /// Export a shareable visibility report
    ///
    /// Examples:
    ///   llmention share myproject.com
    ///   llmention share myproject.com --days 30 > report.md
    ///   llmention share myproject.com --format json > report.json
    Share {
        /// Domain to export
        domain: String,
        /// Number of days of history to include
        #[arg(long, short, default_value = "7")]
        days: u32,
        /// Output format
        #[arg(long, short, value_enum, default_value = "markdown")]
        format: ShareFormat,
    },
    /// Show personal usage stats and trends
    ///
    /// Examples:
    ///   llmention stats myproject.com
    ///   llmention stats myproject.com --days 30
    Stats {
        /// Domain to show stats for (omit to list all tracked domains)
        domain: Option<String>,
        /// Number of days of history
        #[arg(long, short, default_value = "30")]
        days: u32,
    },
    /// Interactive goal-oriented GEO assistant in your terminal
    ///
    /// Examples:
    ///   llmention chat
    ///   llmention chat --models ollama
    Chat,
    /// Print command documentation as markdown
    Docs,
    /// Create config file and show setup instructions
    Config,
    /// Check your setup: config, providers, Ollama connectivity
    Doctor,
    /// Guided beginner flow — prints the recommended steps to get started
    Quickstart,
    /// Interactive setup wizard — configure providers and run your first audit
    ///
    /// Examples:
    ///   llmention init
    Init,
}

#[derive(clap::Subcommand)]
enum ProjectAction {
    /// Add or update a saved project
    Add {
        /// Domain or brand (e.g. myproject.com)
        domain: String,
        /// Product niche
        #[arg(long)]
        niche: Option<String>,
        /// Optional notes
        #[arg(long)]
        notes: Option<String>,
    },
    /// Remove a saved project
    #[command(alias = "rm")]
    Remove {
        /// Domain to remove
        domain: String,
    },
}

#[derive(clap::Subcommand)]
enum PluginAction {
    /// List installed plugins
    List,
    /// Mark a plugin as enabled (adds to config)
    Enable {
        /// Plugin name
        name: String,
    },
    /// Mark a plugin as disabled (removes from config)
    Disable {
        /// Plugin name
        name: String,
    },
}

#[derive(clap::Subcommand)]
enum PromptMarketAction {
    /// List all available community templates
    List,
    /// Search templates by keyword, tag, or name
    Search {
        /// Search query
        query: String,
    },
    /// Install a builtin template as a local plugin you can customize
    Install {
        /// Template name (e.g. rust-crate)
        name: String,
    },
}

#[derive(Clone, clap::ValueEnum)]
enum ShareFormat {
    Markdown,
    Json,
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
                    quiet: cli.quiet,
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
                    quiet: cli.quiet,
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

        Commands::Optimize { domain, niche, competitors, steps, max_rounds, dry_run, auto_apply, plugin } => {
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

            let generate_template_override = resolve_generate_template(plugin.as_deref(), &base_dir);
            let discover_template_override = resolve_discover_template(plugin.as_deref(), &base_dir);

            println!(
                "\n  {}  {}\n  {}  {}\n  {}  {} steps{}{}\n",
                "Optimizing".bold(),
                domain.cyan().bold(),
                "Niche:".dimmed(),
                niche.cyan(),
                "Mode:".dimmed(),
                steps,
                if dry_run { "  [dry-run]".yellow().to_string() } else { String::new() },
                plugin.as_deref().map(|p| format!("  [plugin: {}]", p).yellow().to_string()).unwrap_or_default()
            );

            let opts = OptimizeOptions {
                domain: domain.clone(),
                niche,
                competitors: competitors_list,
                steps,
                max_rounds,
                dry_run,
                verbose: cli.verbose,
                quiet: cli.quiet,
                generate_template_override,
                discover_template_override,
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

        Commands::Generate { prompt, about, niche, output, evaluate, plugin } => {
            let providers = tracker::build_providers_filtered(&config, cli.models.as_deref());
            if providers.is_empty() {
                no_providers_error();
            }

            let about_str = about.as_deref().unwrap_or("").to_string();
            let niche_str = niche.as_deref().unwrap_or("general").to_string();
            let system_prompt_override = resolve_generate_template(plugin.as_deref(), &base_dir);

            println!(
                "\n  {} {}\n  {} {}{}\n",
                "Generating content for:".bold(),
                format!("\"{}\"", prompt).cyan(),
                "Using models:".dimmed(),
                providers.iter().map(|p| p.name()).collect::<Vec<_>>().join(", ").cyan(),
                plugin.as_deref().map(|p| format!("  [plugin: {}]", p).yellow().to_string()).unwrap_or_default()
            );

            let opts = GenerateOptions {
                prompt: prompt.clone(),
                about: about_str.clone(),
                niche: niche_str,
                verbose: cli.verbose,
                system_prompt_override,
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

        Commands::Projects { action } => {
            match action {
                None | Some(ProjectAction::Add { .. }) if matches!(action, None) => {
                    // list
                    run_projects_list(&storage)?;
                }
                Some(ProjectAction::Add { domain, niche, notes }) => {
                    storage.upsert_project(&domain, niche.as_deref(), notes.as_deref())?;
                    println!(
                        "\n  {}  {} saved to projects\n",
                        "✓".green().bold(),
                        domain.cyan()
                    );
                }
                Some(ProjectAction::Remove { domain }) => {
                    if storage.remove_project(&domain)? {
                        println!("\n  {}  {} removed\n", "✓".green().bold(), domain.cyan());
                    } else {
                        println!("\n  {}  {} not found in projects\n", "!".yellow(), domain.cyan());
                    }
                }
                _ => run_projects_list(&storage)?,
            }
        }

        Commands::Watch { domain, niche, interval } => {
            let providers = tracker::build_providers_filtered(&config, cli.models.as_deref());
            if providers.is_empty() {
                no_providers_error();
            }
            let audit_prompts =
                prompts::default_prompts(&domain, niche.as_deref(), None);

            if !cli.quiet {
                println!(
                    "\n  {}  {}  every {} min  — Ctrl+C to stop\n",
                    "Watching".bold(),
                    domain.cyan().bold(),
                    interval
                );
            }

            let mut prev_rate: Option<f64> = fetch_prev_rate(&storage, &domain);
            let dur = std::time::Duration::from_secs(interval * 60);
            loop {
                let track_opts = TrackOptions {
                    verbose: false,
                    concurrency: config.defaults.concurrency,
                    judge: None,
                    quiet: true, // always quiet internally; we print our own summary
                };
                match tracker::run_track(
                    &domain,
                    audit_prompts.clone(),
                    providers.clone(),
                    &storage,
                    &cache,
                    track_opts,
                )
                .await
                {
                    Ok(summary) => {
                        let rate = summary.mention_rate();
                        let ts = chrono::Utc::now().format("%Y-%m-%d %H:%M UTC");
                        let trend = match prev_rate {
                            Some(p) if rate - p > 2.0 => format!(" ↑{:.0}pp", rate - p).green().to_string(),
                            Some(p) if p - rate > 2.0 => format!(" ↓{:.0}pp", p - rate).red().to_string(),
                            Some(_) => " →".dimmed().to_string(),
                            None => String::new(),
                        };
                        let rate_str = format!("{:.0}%", rate);
                        let rate_colored = if rate >= 60.0 {
                            rate_str.green().bold()
                        } else if rate >= 30.0 {
                            rate_str.yellow().bold()
                        } else {
                            rate_str.red().bold()
                        };
                        println!(
                            "  {}  {}  {}{}  ({}/{})",
                            ts.to_string().dimmed(),
                            domain.cyan(),
                            rate_colored,
                            trend,
                            summary.mention_count,
                            summary.total_queries
                        );
                        let _ = storage.touch_project_last_audited(&domain);
                        prev_rate = Some(rate);
                    }
                    Err(e) => eprintln!("  {} {}", "Error:".red().bold(), e),
                }
                tokio::time::sleep(dur).await;
            }
        }

        Commands::Plugins { action } => {
            let installed = plugins::discover_plugins(&base_dir);
            match action {
                None | Some(PluginAction::List) => {
                    println!();
                    println!("  {}  {} installed", "Plugins".bold(), installed.len().to_string().cyan());
                    println!("{}", "─".repeat(64).dimmed());
                    if installed.is_empty() {
                        println!(
                            "\n  No plugins installed. Try:\n  {}\n",
                            "llmention prompts install rust-crate".cyan()
                        );
                    } else {
                        use comfy_table::{Attribute, Cell, Color, ContentArrangement, Table};
                        let mut table = Table::new();
                        table.set_content_arrangement(ContentArrangement::Dynamic);
                        table.set_header(vec![
                            Cell::new("Name").add_attribute(Attribute::Bold),
                            Cell::new("Version").add_attribute(Attribute::Bold),
                            Cell::new("Description").add_attribute(Attribute::Bold),
                            Cell::new("Author").add_attribute(Attribute::Bold),
                        ]);
                        for p in &installed {
                            table.add_row(vec![
                                Cell::new(&p.manifest.meta.name).fg(Color::Cyan),
                                Cell::new(&p.manifest.meta.version).fg(Color::DarkGrey),
                                Cell::new(&p.manifest.meta.description),
                                Cell::new(&p.manifest.meta.author).fg(Color::DarkGrey),
                            ]);
                        }
                        println!();
                        println!("{table}");
                    }
                    println!(
                        "\n  {}  Use {} to apply a plugin\n",
                        "Tip".yellow().bold(),
                        "--plugin <name>".cyan()
                    );
                }
                Some(PluginAction::Enable { name }) => {
                    if plugins::find_plugin(&base_dir, &name).is_some() {
                        println!("\n  {}  Plugin {} is installed and ready.\n  Use {} to apply it.\n",
                            "✓".green().bold(), name.cyan(),
                            format!("--plugin {}", name).cyan());
                    } else {
                        println!("\n  {}  Plugin {} is not installed. Run:\n  {}\n",
                            "!".yellow(), name.cyan(),
                            format!("llmention prompts install {}", name).cyan());
                    }
                }
                Some(PluginAction::Disable { name }) => {
                    println!("\n  {}  Plugin {} will not be auto-applied (pass --plugin to use it explicitly).\n",
                        "✓".green().bold(), name.cyan());
                }
            }
        }

        Commands::Prompts { action } => match action {
            PromptMarketAction::List => {
                use comfy_table::{Attribute, Cell, Color, ContentArrangement, Table};
                println!();
                println!("  {}  {} available", "Community Templates".bold(),
                    registry::BUILTIN_TEMPLATES.len().to_string().cyan());
                println!("{}", "─".repeat(64).dimmed());
                println!();
                let mut table = Table::new();
                table.set_content_arrangement(ContentArrangement::Dynamic);
                table.set_header(vec![
                    Cell::new("Name").add_attribute(Attribute::Bold),
                    Cell::new("Description").add_attribute(Attribute::Bold),
                    Cell::new("Tags").add_attribute(Attribute::Bold),
                ]);
                for t in registry::BUILTIN_TEMPLATES {
                    table.add_row(vec![
                        Cell::new(t.name).fg(Color::Cyan),
                        Cell::new(t.description),
                        Cell::new(t.tags.join(", ")).fg(Color::DarkGrey),
                    ]);
                }
                println!("{table}");
                println!(
                    "\n  {}  llmention prompts install <name>\n",
                    "→".cyan()
                );
            }
            PromptMarketAction::Search { query } => {
                use comfy_table::{Attribute, Cell, Color, ContentArrangement, Table};
                let results = registry::search_templates(&query);
                println!();
                println!("  {}  {} match(es) for \"{}\"",
                    "Search".bold(), results.len().to_string().cyan(), query);
                println!("{}", "─".repeat(64).dimmed());
                if results.is_empty() {
                    println!("\n  No templates matched your query.\n");
                } else {
                    println!();
                    let mut table = Table::new();
                    table.set_content_arrangement(ContentArrangement::Dynamic);
                    table.set_header(vec![
                        Cell::new("Name").add_attribute(Attribute::Bold),
                        Cell::new("Description").add_attribute(Attribute::Bold),
                        Cell::new("Tags").add_attribute(Attribute::Bold),
                    ]);
                    for t in results {
                        table.add_row(vec![
                            Cell::new(t.name).fg(Color::Cyan),
                            Cell::new(t.description),
                            Cell::new(t.tags.join(", ")).fg(Color::DarkGrey),
                        ]);
                    }
                    println!("{table}");
                    println!();
                }
            }
            PromptMarketAction::Install { name } => {
                match registry::find_template(&name) {
                    None => {
                        println!("\n  {}  Template {} not found. Run {} to see available templates.\n",
                            "✗".red().bold(), name.cyan(), "llmention prompts list".cyan());
                    }
                    Some(info) => {
                        let plugin_dir = base_dir.join("plugins").join(&name);
                        std::fs::create_dir_all(&plugin_dir)?;

                        let manifest = format!(
                            "[meta]\nname = \"{}\"\nversion = \"1.0.0\"\ndescription = \"{}\"\nauthor = \"{}\"\ntags = [{}]\n\n[templates]\n{}{}\n",
                            info.name,
                            info.description,
                            info.author,
                            info.tags.iter().map(|t| format!("\"{}\"", t)).collect::<Vec<_>>().join(", "),
                            builtin::generate_template(&name).map(|_| "generate = \"generate.prompt.md\"\n").unwrap_or(""),
                            builtin::discover_template(&name).map(|_| "discover = \"discover.prompt.md\"\n").unwrap_or(""),
                        );
                        std::fs::write(plugin_dir.join("plugin.toml"), &manifest)?;

                        if let Some(gen_tpl) = builtin::generate_template(&name) {
                            std::fs::write(plugin_dir.join("generate.prompt.md"), gen_tpl)?;
                        }
                        if let Some(disc_tpl) = builtin::discover_template(&name) {
                            std::fs::write(plugin_dir.join("discover.prompt.md"), disc_tpl)?;
                        }

                        println!("\n  {}  Installed {} to {}\n",
                            "✓".green().bold(),
                            name.cyan(),
                            plugin_dir.display().to_string().dimmed());
                        println!("  {}  Edit templates at:", "Tip".yellow().bold());
                        println!("     {}", plugin_dir.display().to_string().cyan());
                        println!("\n  {}  Use it with:\n  {}\n",
                            "→".cyan(),
                            format!("llmention generate \"...\" --plugin {} --about \"...\"", name).cyan());
                    }
                }
            }
        },

        Commands::Share { domain, days, format } => {
            let results = storage.query_domain(&domain, days)?;
            if results.is_empty() {
                println!("\n  {}  No data for {}. Run {} first.\n",
                    "!".yellow(),
                    domain.cyan(),
                    format!("llmention audit {}", domain).cyan());
            } else {
                match format {
                    ShareFormat::Json => print!("{}", report::render_share_json(&domain, &results, days)),
                    ShareFormat::Markdown => print!("{}", report::render_share_markdown(&domain, &results, days)),
                }
            }
        }

        Commands::Stats { domain, days } => {
            match domain {
                None => {
                    let domains = storage.list_domains()?;
                    println!();
                    println!("  {}  {} domain(s) tracked", "Stats".bold(), domains.len().to_string().cyan());
                    println!("{}", "─".repeat(64).dimmed());
                    if domains.is_empty() {
                        println!("\n  No data yet. Run {} to start tracking.\n",
                            "llmention audit <domain>".cyan());
                    } else {
                        for d in &domains {
                            println!("  {}  {}", "·".dimmed(), d.cyan());
                        }
                        println!("\n  {}  llmention stats <domain>\n", "→".cyan());
                    }
                }
                Some(domain) => {
                    let stats = storage.domain_stats(&domain, days)?;
                    report::print_stats(&domain, &stats, days);
                }
            }
        }

        Commands::Chat => {
            let providers = tracker::build_providers_filtered(&config, cli.models.as_deref());
            if providers.is_empty() {
                no_providers_error();
            }
            tui::chat::run(providers).await?;
        }

        Commands::Docs => {
            print!("{}", generate_docs());
        }

        Commands::Init => run_init()?,

        Commands::Config => run_config_command()?,

        Commands::Doctor => run_doctor(&config, &base_dir).await?,

        Commands::Quickstart => run_quickstart()?,
    }

    Ok(())
}

// ── helpers ──────────────────────────────────────────────────────────────────

fn print_welcome() {
    println!("{}", BANNER.cyan().dimmed());
    println!("{}", "━".repeat(62).dimmed());
    println!("  {}", TAGLINE);
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
    println!("    {}  gemini     — gemini-2.0-flash, gemini-1.5-pro  (Google)", "·".dimmed());
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

    // Gemini
    match &config.providers.gemini {
        Some(c) if c.enabled => {
            any_enabled = true;
            println!("  {}  gemini    {} ({})", "✓".green(), "enabled".green(), c.model.dimmed());
        }
        Some(_) => println!("  {}  gemini    disabled", "–".dimmed()),
        None => println!("  {}  gemini    not configured", "–".dimmed()),
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

fn run_quickstart() -> Result<()> {
    println!();
    println!("{}", "LLMention Quickstart".bold());
    println!("{}", "━".repeat(56).dimmed());
    println!();
    println!("  {}  {}", "1.".bold().to_string(), "Create config".bold());
    println!("      {}", "llmention config".cyan());
    println!();
    println!("  {}  {}", "2.".bold().to_string(), "Add your API key (or enable Ollama for free)".bold());
    println!("      {}", "Edit ~/.llmention/config.toml".cyan());
    println!();
    println!("  {}  {}", "3.".bold().to_string(), "Verify your setup".bold());
    println!("      {}", "llmention doctor".cyan());
    println!();
    println!("  {}  {}", "4.".bold().to_string(), "Run your first audit".bold());
    println!("      {}", "llmention audit myproject.com --niche \"your niche\"".cyan());
    println!();
    println!("  {}  {}", "5.".bold().to_string(), "Let the agent optimize (optional)".bold());
    println!("      {}", "llmention optimize myproject.com --niche \"your niche\" --auto-apply".cyan());
    println!();
    println!("{}", "─".repeat(56).dimmed());
    println!();
    println!("  {}  Need help? Run {} for full documentation.",
        "Tip".yellow().bold(),
        "llmention docs".cyan()
    );
    println!();
    Ok(())
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

/// Resolve a generate system prompt template from a named plugin.
/// Checks installed plugins first, then falls back to builtins.
fn resolve_generate_template(name: Option<&str>, config_dir: &PathBuf) -> Option<String> {
    let name = name?;
    if let Some(plugin) = plugins::find_plugin(config_dir, name) {
        if let Some(tpl) = plugin.generate_template() {
            return Some(tpl);
        }
    }
    builtin::generate_template(name).map(|s| s.to_string())
}

/// Resolve a discover system prompt template from a named plugin.
fn resolve_discover_template(name: Option<&str>, config_dir: &PathBuf) -> Option<String> {
    let name = name?;
    if let Some(plugin) = plugins::find_plugin(config_dir, name) {
        if let Some(tpl) = plugin.discover_template() {
            return Some(tpl);
        }
    }
    builtin::discover_template(name).map(|s| s.to_string())
}

fn generate_docs() -> String {
    let mut out = String::from("# LLMention — Command Reference\n\n");
    out.push_str("Local-first GEO (Generative Engine Optimization) agent for indie hackers.\n\n");
    out.push_str("---\n\n");

    let commands = [
        ("audit", "Quick visibility scan using smart default prompts.",
         "llmention audit myproject.com\nllmention audit myproject.com --niche \"Rust CLI tool\"\nllmention audit myproject.com --models ollama"),
        ("track", "Run custom prompts from a file and record brand mentions.",
         "llmention track myproject.com --prompts prompts.txt\nllmention track myproject.com --prompts prompts.json --models anthropic"),
        ("report", "Show mention history and trends from the local database.",
         "llmention report myproject.com\nllmention report myproject.com --days 30\nllmention report myproject.com --export csv > results.csv"),
        ("generate", "Generate GEO-optimized markdown content for a target query.",
         "llmention generate \"best rust cli tool\" --about \"myproject.io is a ...\"\nllmention generate \"...\" --plugin rust-crate --about \"...\"\nllmention generate \"...\" --evaluate"),
        ("optimize", "5-step autonomous GEO agent: discover, audit, generate, refine, evaluate.",
         "llmention optimize myproject.com --niche \"Rust CLI tool\"\nllmention optimize myproject.com --niche \"...\" --steps 5 --auto-apply\nllmention optimize myproject.com --niche \"...\" --max-rounds 3\nllmention optimize myproject.com --niche \"...\" --plugin rust-crate"),
        ("chat", "Structured TUI assistant — state a goal, get a guided GEO plan.",
         "llmention chat\nllmention chat --models ollama"),
        ("projects", "Manage saved domain + niche pairs.",
         "llmention projects\nllmention projects add myproject.com --niche \"Rust CLI tool\"\nllmention projects remove myproject.com"),
        ("watch", "Background polling audit on a fixed interval.",
         "llmention watch myproject.com --niche \"Rust CLI tool\"\nllmention watch myproject.com --interval 30 --models ollama"),
        ("stats", "Personal usage trends and per-day breakdown.",
         "llmention stats\nllmention stats myproject.com\nllmention stats myproject.com --days 30"),
        ("share", "Export a shareable visibility report.",
         "llmention share myproject.com\nllmention share myproject.com --days 30 > report.md\nllmention share myproject.com --format json > report.json"),
        ("prompts", "Browse and install community prompt templates.",
         "llmention prompts list\nllmention prompts search rust\nllmention prompts install rust-crate"),
        ("plugins", "Manage installed plugins.",
         "llmention plugins\nllmention plugins enable rust-crate\nllmention plugins disable rust-crate"),
        ("config", "Create ~/.llmention/config.toml and show setup instructions.", "llmention config"),
        ("doctor", "Verify config, providers, and Ollama connectivity.", "llmention doctor"),
        ("docs", "Print this command reference as markdown.", "llmention docs > COMMANDS.md"),
    ];

    for (name, desc, examples) in &commands {
        out.push_str(&format!("## `{}`\n\n{}\n\n", name, desc));
        out.push_str("```bash\n");
        out.push_str(examples);
        out.push_str("\n```\n\n");
    }

    out.push_str("---\n\n");
    out.push_str("## Global Flags\n\n");
    out.push_str("| Flag | Description |\n");
    out.push_str("|------|-------------|\n");
    out.push_str("| `--models openai,anthropic` | Comma-separated provider list |\n");
    out.push_str("| `--verbose` | Show raw LLM response previews |\n");
    out.push_str("| `--quiet` | Suppress progress output (CI-friendly) |\n\n");
    out.push_str("---\n\n");
    out.push_str("_Generated by `llmention docs` — [LLMention](https://github.com/wiramahendra/llMention)_\n");
    out
}

fn run_projects_list(storage: &Storage) -> anyhow::Result<()> {
    use comfy_table::{Attribute, Cell, Color, ContentArrangement, Table};
    let projects = storage.list_projects()?;
    println!();
    println!(
        "  {}  {} saved",
        "Projects".bold(),
        projects.len().to_string().cyan()
    );
    println!("{}", "─".repeat(64).dimmed());
    if projects.is_empty() {
        println!(
            "\n  No projects yet. Add one:\n  {}\n",
            "llmention projects add myproject.com --niche \"your niche\"".cyan()
        );
        return Ok(());
    }
    println!();
    let mut table = Table::new();
    table.set_content_arrangement(ContentArrangement::Dynamic);
    table.set_header(vec![
        Cell::new("Domain").add_attribute(Attribute::Bold),
        Cell::new("Niche").add_attribute(Attribute::Bold),
        Cell::new("Last Audited").add_attribute(Attribute::Bold),
        Cell::new("Notes").add_attribute(Attribute::Bold),
    ]);
    for p in &projects {
        let last = p
            .last_audited
            .as_deref()
            .and_then(|ts| chrono::DateTime::parse_from_rfc3339(ts).ok())
            .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
            .unwrap_or_else(|| "never".to_string());
        table.add_row(vec![
            Cell::new(&p.domain).fg(Color::Cyan),
            Cell::new(p.niche.as_deref().unwrap_or("—")),
            Cell::new(last).fg(Color::DarkGrey),
            Cell::new(p.notes.as_deref().unwrap_or("—")).fg(Color::DarkGrey),
        ]);
    }
    println!("{table}");
    println!(
        "\n  {}  llmention audit <domain>  or  llmention optimize <domain> --niche <niche>\n",
        "Tip".yellow().bold()
    );
    Ok(()
    )
}
