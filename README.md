# LLMention

[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Rust 1.75+](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org)
[![Version](https://img.shields.io/badge/version-0.2.0-blue.svg)](https://github.com/wiramahendra/llMention/releases)

**The terminal-native GEO agent for indie hackers and open-source maintainers.**

LLMention tracks, generates, and optimizes your brand's AI visibility in ChatGPT, Claude, Grok, Perplexity, and any LLM you configure — privately, locally, no SaaS.

```
  Mention rate   67%  (8/12 queries)  (↑ 24pp vs last run)
  Citations      2
  Models         2/3  (openai, anthropic)
```

---

## Why LLMention?

- **Private.** Your prompts never leave your machine. No telemetry, no sign-up, no cloud DB.
- **Unlimited.** Use your own API keys or run 100% locally with [Ollama](https://ollama.com) — no per-query pricing.
- **Agentic.** The `optimize` command autonomously discovers weak topics, generates LLM-citable content, and projects your visibility lift.
- **Scriptable.** `--quiet` flag and clean exit codes for CI pipelines and automation.

|                    | LLMention          | Enterprise GEO tools |
|--------------------|--------------------|----------------------|
| Price              | Free / open-source | $200–$2 000/mo       |
| Data stays local   | ✓                  | ✗ (their servers)    |
| Content generation | ✓ built-in         | ✗                    |
| Agentic optimize   | ✓ 5-step agent     | ✗                    |
| Project manager    | ✓ SQLite           | Limited              |
| Watch mode         | ✓ background poll  | ✗                    |
| Desktop GUI        | ✓ optional Tauri   | Web dashboard        |
| Ollama support     | ✓ fully local      | ✗                    |
| Binary size        | 7.3 MB             | —                    |

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
# Binary at target/release/llmention (7.3 MB)
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
# 1. Create config
llmention config

# 2. Edit ~/.llmention/config.toml — add API key or enable Ollama

# 3. Verify
llmention doctor

# 4. Run your first audit
llmention audit myproject.com --niche "Rust CLI tool"

# 5. Save it as a project
llmention projects add myproject.com --niche "Rust CLI tool"

# 6. Let the agent optimize
llmention optimize myproject.com --niche "Rust CLI tool" --auto-apply
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

### `config` / `doctor`

```bash
llmention config     # create ~/.llmention/config.toml
llmention doctor     # verify config, providers, and Ollama connectivity
```

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

## How It Works

| Layer | Module | Purpose |
|-------|--------|---------|
| Tracking | `tracker.rs` | Concurrent query orchestrator (semaphore-limited) |
| Parsing | `parser.rs` | Rule-based mention / citation / sentiment detection |
| Cache | `cache.rs` | 24-hour SHA-256 keyed file cache |
| Storage | `storage.rs` | SQLite (`~/.llmention/mentions.db`) with projects table |
| Generation | `geo/generator.rs` | GEO content via LlmProvider + embedded prompt templates |
| Evaluation | `geo/evaluator.rs` | Citability scoring using structured LLM eval |
| Agent | `agent/optimizer.rs` | 5-step orchestrator (discover → audit → generate → score) |
| Discovery | `agent/prompt_discovery.rs` | LLM-based high-intent prompt generation |

---

## Project Structure

```
src/
  bin/llmention.rs        CLI entrypoint (clap, 9 commands)
  agent/
    optimizer.rs          5-step GEO agent
    plan.rs               OptimizationPlan structs
    prompt_discovery.rs   LLM-driven prompt discovery
  geo/
    generator.rs          GEO content generation
    evaluator.rs          Before/after citability scoring
    prompts.rs            Template loading, default_prompts()
    templates/            Embedded .prompt.md files
  providers/              LlmProvider trait + OpenAI, Anthropic, xAI, Perplexity, Ollama
  tracker.rs              Parallel query orchestrator
  parser.rs               Mention/citation/sentiment detection
  cache.rs                24-hour file cache
  storage.rs              SQLite (mentions + projects tables)
  report.rs               Terminal output + CSV/Markdown export
  types.rs                Shared types

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
```

To add a new provider: implement `LlmProvider` in `src/providers/`, add config fields in `src/config.rs`, wire it in `tracker::build_providers`.

PRs welcome. Please keep the binary under 10 MB (`cargo build --release && ls -lh target/release/llmention`).

---

## Roadmap

| Phase | Feature | Status |
|-------|---------|--------|
| 1 | `audit`, `track`, `report`, `config`, `doctor` | ✅ Done |
| 2 | `generate` — GEO-optimized markdown | ✅ Done |
| 3 | `optimize` — 5-step GEO agent | ✅ Done |
| 3 | `projects`, `watch`, `--quiet`, desktop app skeleton | ✅ Done |
| 4 | Prompt marketplace (community prompt packs) | Planned |
| 4 | Plugin system for custom providers | Planned |
| 5 | Self-hosted web dashboard | Planned |

---

## License

MIT — see [LICENSE](LICENSE).
