# LLMention

[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Rust 1.75+](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org)
[![Version](https://img.shields.io/badge/version-0.3.0-blue.svg)](https://github.com/wiramahendra/llMention/releases)

> **The private, local-first GEO companion for indie builders — track, generate, and optimize your visibility in AI answers.**

LLMention tracks, generates, and optimizes your brand's AI visibility in ChatGPT, Claude, Grok, Perplexity, and any LLM you configure — privately, locally, no SaaS, no subscriptions.

```
  Mention rate   67%  (8/12 queries)  (↑ 24pp vs last run)
  Citations      2
  Models         2/3  (openai, anthropic)
```

---

## The Honest Pitch

- **Private.** Your prompts never leave your machine. No telemetry, no sign-up, no cloud DB.
- **Local-first.** Use your own API keys or run 100% free with [Ollama](https://ollama.com) — no per-query pricing.
- **Developer-first.** Clean CLI, scriptable with `--quiet`, native binary (~7 MB).
- **No lock-in.** Extensible via plugins. Your data stays in SQLite on your machine.

**What LLMention does:** It measures how often LLMs mention your brand and generates optimized content to improve those odds.

**What LLMention doesn't do:** It doesn't guarantee citations. GEO results depend on your content quality and model behavior. We help you measure and improve — success requires ongoing effort.

|                    | LLMention          | The Prompting Company | Enterprise GEO tools |
|--------------------|--------------------|-----------------------|----------------------|
| Managed creation & routing | ✗ | ✓ | ✓ |
| Self-serve tools  | ✓                  | ✗                    | ✗                   |
| Price              | Free / open-source | $50–$500/mo          | $200–$2 000/mo      |
| Data stays local   | ✓                  | ✗ (their servers)    | ✗ (their servers)   |
| Content generation | ✓ built-in         | ✓                    | ✗                   |
| Ollama support     | ✓ fully local      | ✗                    | ✗                   |

---

## Installation

### Pre-built binaries (macOS, Linux, Windows)

```bash
# macOS / Linux
curl -fsSL https://raw.githubusercontent.com/wiramahendra/llMention/main/scripts/install.sh | bash

# Windows (PowerShell)
irm https://raw.githubusercontent.com/wiramahendra/llMention/main/scripts/install.ps1 | iex
```

### Cargo (build from source)

```bash
cargo install --git https://github.com/wiramahendra/llMention
```

### From source

```bash
git clone https://github.com/wiramahendra/llMention
cd llmention
cargo build --release
# Binary at target/release/llmention (7.5 MB)
```

### Desktop App (optional)

Requires [Node.js 18+](https://nodejs.org) and [Rust](https://rustup.rs).

```bash
cd tauri-app
npm install
npm run tauri dev       # development
npm run tauri build     # release build
```

The desktop app wraps the same core library — identical results to the CLI.

---

## Quick Start

```bash
# Guided interactive walkthrough
llmention quickstart
```

Or manually:

```bash
llmention config                                         # 1. create config
# edit ~/.llmention/config.toml — add API key or enable Ollama
llmention doctor                                         # 2. verify setup
llmention audit myproject.com --niche "Rust CLI tool"   # 3. first scan
llmention projects add myproject.com --niche "Rust CLI tool"  # 4. save project
llmention optimize myproject.com --niche "Rust CLI tool" --auto-apply  # 5. improve
```

> **Zero-cost option:** `ollama pull llama3.2` → set `enabled = true` under `[providers.ollama]` → use `--models ollama`

---

## Commands

### `optimize` — Full GEO agent

Runs a 5-step autonomous workflow: **discover → audit → identify → generate → evaluate**

```bash
llmention optimize igrisinertial.com --niche "deterministic edge runtime"
llmention optimize myproject.com --niche "rust cli tool" --competitors "ripgrep,fd" --steps 5
llmention optimize myproject.com --niche "observability" --dry-run
llmention optimize myproject.com --niche "edge AI runtime" --auto-apply
```

**Example output:**
```
  Optimizing  igrisinertial.com
  Niche:      deterministic edge runtime

  [1/5]  Discovering high-intent prompts…
         → Found 12 prompts

  [2/5]  Auditing current visibility…
         → Mention rate: 0%  (0/12)

  [3/5]  Identifying optimization opportunities…
         → 12 weak topics — targeting 3

  [4/5]  Generating optimized content…
         → [1/3] "alternatives to ros2 for robotics"…  ✓ (anthropic)

  [5/5]  Evaluating citability…
         → [anthropic] ✓ 92%  — alternatives to ros2 for robotics

  ════════════════════════════════════════════════════════════════
  Optimization Plan  igrisinertial.com
  ════════════════════════════════════════════════════════════════

  Current visibility     0%   (0 queries across 12 topics)
  Projected citability  86%   (+86pp on optimized topics)

  ┌─────────────────────────────────┬────────────┬─────────────────────────────┐
  │ Prompt                          │ Citability │ File                        │
  ├─────────────────────────────────┼────────────┼─────────────────────────────┤
  │ alternatives to ros2            │ ✓ 92%      │ geo/alternatives-to-ros2.md │
  └─────────────────────────────────┴────────────┴─────────────────────────────┘

  →  git add geo/ && git commit -m "docs: add GEO-optimized content"
  →  llmention audit igrisinertial.com --niche "deterministic edge runtime"
```

### `generate` — Single-query content generation

```bash
llmention generate "best deterministic runtime for edge AI" \
  --about "igrisinertial.com is a deterministic, failure-resilient runtime" \
  --niche "edge robotics"

llmention generate "what is igrisinertial" --about "..." --output geo/what-is.md
llmention generate "..." --about "..." --evaluate        # before/after visibility estimate
```

### `audit` — Quick visibility scan

```bash
llmention audit myproject.com
llmention audit myproject.com --niche "observability tool" --competitor datadog
llmention audit myproject.com --models openai,ollama
llmention audit myproject.com --judge     # local LLM re-evaluates each response
llmention audit myproject.com --quiet     # CI-friendly minimal output
```

### `track` — Custom prompts

```bash
llmention track myproject.com --prompts prompts.txt
llmention track myproject.com --prompts prompts.json --models anthropic
```

### `projects` — Saved domain/niche pairs

```bash
llmention projects                                                # list
llmention projects add myproject.com --niche "Rust CLI tool"     # save
llmention projects add myproject.com --notes "v2 launch: Apr 26" # update
llmention projects remove myproject.com                          # delete
```

### `watch` — Background periodic audits

Runs an audit on a timer. Useful for dashboards or CI health checks.

```bash
llmention watch myproject.com --niche "Rust CLI tool"            # every 60 min
llmention watch myproject.com --interval 30 --models ollama      # every 30 min, local
llmention watch myproject.com --interval 1440                    # daily
```

**Output format (one line per run):**
```
  2026-04-19 08:30 UTC  myproject.com  67%  ↑4pp  (8/12)
  2026-04-19 09:30 UTC  myproject.com  71%  ↑4pp  (9/12)
```

### `report` — History & trends

```bash
llmention report myproject.com
llmention report myproject.com --days 30
llmention report myproject.com --export csv > results.csv
llmention report myproject.com --export markdown > report.md
```

### `stats` — Usage trends

```bash
llmention stats                        # list all tracked domains
llmention stats myproject.com          # per-day mention breakdown
llmention stats myproject.com --days 30
```

### `share` — Shareable reports

Export a visibility snapshot to share with your team or on social:

```bash
llmention share myproject.com                      # markdown to stdout
llmention share myproject.com --days 30 > report.md
llmention share myproject.com --format json > report.json
```

### `prompts` — Community template marketplace

```bash
llmention prompts list                  # browse all available templates
llmention prompts search rust           # search by keyword or tag
llmention prompts install rust-crate    # install & customize locally
```

### `plugins` — Plugin management

```bash
llmention plugins                       # list installed plugins
llmention plugins enable rust-crate     # mark as active
```

Once installed, apply a plugin with `--plugin`:

```bash
llmention generate "best rust cli tool" --plugin rust-crate --about "myproject.io is..."
llmention optimize myproject.com --niche "Rust CLI" --plugin rust-crate --auto-apply
```

### `docs` — Command reference

```bash
llmention docs                         # print full docs as markdown
llmention docs > COMMANDS.md           # save to file
```

### `quickstart` / `docs` / `config` / `doctor`

```bash
llmention quickstart    # guided step-by-step beginner flow
llmention docs          # full command reference as markdown
llmention docs > COMMANDS.md
llmention config        # create ~/.llmention/config.toml
llmention doctor        # verify config, providers, Ollama connectivity
```

---

## Plugin System

LLMention supports **prompt plugins** — reusable template packs that specialize content generation for specific niches (Rust crates, Python packages, SaaS products, etc.).

### Built-in templates

| Name | Best for |
|------|----------|
| `rust-crate` | Rust crates and CLI tools |
| `python-package` | Python packages (PyPI) |
| `saas-product` | SaaS products and web apps |
| `open-source` | Any open-source project |
| `technical-blog` | Developer blogs and tutorials |
| `personal-brand` | Indie hackers and personal brands |

### Installing a template

```bash
llmention prompts install rust-crate
# Files written to ~/.llmention/plugins/rust-crate/
# Edit generate.prompt.md to customize
```

### Creating your own plugin

```
~/.llmention/plugins/my-plugin/
  plugin.toml           # name, version, description, tags
  generate.prompt.md    # system prompt for content generation
  discover.prompt.md    # system prompt for prompt discovery (optional)
```

`plugin.toml`:
```toml
[meta]
name = "my-plugin"
version = "1.0.0"
description = "GEO for my niche"
author = "your-name"

[templates]
generate = "generate.prompt.md"
discover  = "discover.prompt.md"
```

Templates support `{about}`, `{niche}`, `{domain}`, `{competitors}` variables.

See [CONTRIBUTING.md](CONTRIBUTING.md) for the full plugin authoring guide.

---

## CI / Scripting

Use `--quiet` to suppress progress output and get machine-readable results:

```bash
# In a shell script
RATE=$(llmention audit myproject.com --quiet 2>/dev/null | grep "Mention rate" | grep -o "[0-9]*%")

# In GitHub Actions
- name: Check GEO visibility
  run: llmention audit ${{ env.DOMAIN }} --quiet --models ollama
```

---

## Configuration

Config file: `~/.llmention/config.toml` — run `llmention config` to create it.

```toml
[providers.openai]
api_key     = "sk-..."
model       = "gpt-4o-mini"
enabled     = true
temperature = 0          # deterministic, cacheable

[providers.anthropic]
api_key     = "sk-ant-..."
model       = "claude-3-5-haiku-20241022"
enabled     = true
temperature = 0

[providers.xai]
api_key     = "xai-..."
model       = "grok-2-latest"
enabled     = false

[providers.perplexity]
api_key     = "pplx-..."
model       = "sonar"
enabled     = false

# Free, unlimited local inference
[providers.ollama]
base_url  = "http://localhost:11434"
model     = "llama3.2"
enabled   = false

[judge]
enabled   = false
base_url  = "http://localhost:11434"
model     = "llama3.2"

[defaults]
days        = 7
concurrency = 5
```

---

## How It Actually Works

LLMention operates in two phases:

### 1. Measurement (audit, track, watch)
The tool sends prompts about your brand to configured LLMs and parses responses for:
- **Mentions** — does the model mention your brand?
- **Citations** — does it cite your website/content?
- **Sentiment** — positive, neutral, or negative?

Results are stored locally in SQLite. Run audits over time to track trends.

### 2. Optimization (generate, optimize)
- **`generate`**: Creates LLM-citable markdown content for a target query
- **`optimize`**: Autonomous 5-step agent that discovers weak topics → audits → generates content → evaluates citability

**Important:** LLMention improves the *probability* of mentions and citations. It cannot guarantee them. GEO success depends on:
- Your content quality and relevance
- Model training data and behavior
- Competitor presence in your niche
- Ongoing iteration and testing

---

## Built With

- **Rust** — native binary, no runtime required
- **Tauri** — optional desktop GUI
- **SQLite** — local data storage
- **Ollama** — free local LLM inference
- **clap** — CLI argument parsing
- **tokio** — async runtime

---

## Project Structure

```
src/
  bin/llmention.rs        CLI entrypoint (clap, 14 commands)
  agent/
    optimizer.rs          5-step GEO agent
    plan.rs               OptimizationPlan structs
    prompt_discovery.rs   LLM-driven prompt discovery
  geo/
    generator.rs          GEO content generation (plugin-aware)
    evaluator.rs          Before/after citability scoring
    prompts.rs            Template loading, default_prompts()
    templates/            Embedded .prompt.md files
  marketplace/
    registry.rs           Built-in template catalog (6 niches)
    builtin.rs            Embedded template strings
  plugins/
    manifest.rs           PluginManifest / PluginMeta structs
    loader.rs             Plugin discovery from ~/.llmention/plugins/
  providers/              LlmProvider trait + OpenAI, Anthropic, xAI, Perplexity, Ollama
  tracker.rs              Parallel query orchestrator
  parser.rs               Mention/citation/sentiment detection
  cache.rs                24-hour file cache
  storage.rs              SQLite (mentions + projects + stats)
  report.rs               Terminal output + CSV/Markdown/JSON export
  types.rs                Shared types

templates/community/      Example community plugins (submit PRs here)
  rust-crate/             Rust crate optimizer template

tauri-app/                Optional desktop GUI (Tauri v2 + React)
  src/                    React frontend (TypeScript)
  src-tauri/              Rust backend (Tauri commands)

scripts/
  install.sh              Unix installer
  install.ps1             Windows installer

.github/workflows/
  release.yml             Multi-platform GitHub Releases CI
```

---

## Contributing

```bash
cargo test        # 23 unit tests
cargo clippy
cargo build --release
ls -lh target/release/llmention   # must stay under 10 MB
```

To add a new provider: implement `LlmProvider` in `src/providers/`, add config fields in `src/config.rs`, wire it in `tracker::build_providers`.

To add a community template: see [CONTRIBUTING.md](CONTRIBUTING.md) — create a folder under `templates/community/<name>/` with a `plugin.toml` and prompt files.

PRs welcome.

---

## Roadmap

| Phase | Feature | Status |
|-------|---------|--------|
| 1 | `audit`, `track`, `report`, `config`, `doctor` | ✅ Done |
| 2 | `generate` — GEO-optimized markdown | ✅ Done |
| 3 | `optimize` — 5-step GEO agent | ✅ Done |
| 3 | `projects`, `watch`, `--quiet`, desktop app skeleton | ✅ Done |
| 4 | `prompts`, `plugins`, `share`, `stats`, `docs` | ✅ Done |
| 4 | Plugin system + community template marketplace | ✅ Done |
| 5 | `quickstart`, launch polish, EXAMPLES.md | ✅ Done |
| 6 | Self-hosted web dashboard | Planned |
| 6 | Community prompt registry (web-hosted) | Planned |

---

## License

MIT — see [LICENSE](LICENSE).
