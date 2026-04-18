# LLMention

[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Rust 1.75+](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org)

**Know exactly when and where LLMs mention your brand — local-first, private, no SaaS.**

Run `llmention audit myproject.com` weekly and instantly see:

```
  Mention rate   67%  (8/12 queries)  (↑ 24pp vs last run)
  Citations      2
  Models         2/3  (openai, anthropic)
```

…with concrete GEO tips like *"Add a comparison table"* or *"Move your entity definition to the first sentence"*.

---

## Why LLMention?

- **Private.** Your prompts never leave your machine. No telemetry, no sign-up, no cloud DB.
- **Unlimited.** Use your own API keys or run 100% locally with [Ollama](https://ollama.com) — no per-query pricing.
- **Actionable.** Colored results table + rule-based GEO tips after every run. Not just numbers.

|                  | LLMention          | Enterprise GEO tools |
|------------------|--------------------|----------------------|
| Price            | Free / open-source | $200–$2 000/mo       |
| Data stays local | ✓                  | ✗ (their servers)    |
| Custom prompts   | ✓ any file         | Limited templates    |
| Ollama support   | ✓ fully local      | ✗                    |
| Single binary    | ✓ 9.6 MB           | Web dashboard        |

---

## Quick Start

```bash
# 1. Install
cargo install --git https://github.com/wiramahendra/llMention

# 2. Create config
llmention config

# 3. Edit ~/.llmention/config.toml — add an API key or enable Ollama
#    (see "Configuration" below)

# 4. Verify your setup
llmention doctor

# 5. Run your first audit
llmention audit myproject.com --niche "Rust CLI tool"
```

> **Zero-cost option:** Install [Ollama](https://ollama.com), run `ollama pull llama3.2`,
> set `enabled = true` under `[providers.ollama]` in config, and use
> `llmention audit myproject.com --models ollama`.

---

## Commands

### `audit` — Quick scan (start here)

Generates 12 smart prompts and queries all enabled models. No file needed.

```bash
llmention audit myproject.com
llmention audit myproject.com --niche "observability tool" --competitor datadog
llmention audit myproject.com --models openai,ollama
llmention audit myproject.com --judge     # local LLM re-evaluates each response
```

### `track` — Custom prompts

Load your own prompt list from a `.txt` (one per line) or `.json` array file.

```bash
llmention track myproject.com --prompts prompts.txt
llmention track myproject.com --prompts prompts.json --models anthropic
```

**`prompts.txt` example:**
```
what is myproject
best lightweight monitoring tool 2026
myproject vs prometheus
is myproject production ready
how to install myproject on Ubuntu
```

### `report` — History & trends

Query the local SQLite history. Supports CSV and Markdown export.

```bash
llmention report myproject.com
llmention report myproject.com --days 30
llmention report myproject.com --export csv > results.csv
llmention report myproject.com --export markdown > report.md
```

### `config` — Setup

Creates `~/.llmention/config.toml` with a commented example on first run.

```bash
llmention config
```

### `doctor` — Verify setup

Checks config, database, cache directory, provider status, and Ollama connectivity.

```bash
llmention doctor
```

---

## Example Output

```
  Auditing myproject.com — 12 prompts × 3 model(s)

  ✓  [ 1/36] [openai] what is myproject
  –  [ 2/36] [openai] best developer tool 2026
  ✓  [ 3/36] [anthropic] myproject review
  ...

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  LLMention Report:  myproject.com
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

  Mention rate   67%  (8/12 queries)  (↑ 24pp vs last run)
  Citations      2
  Models         2/3  (openai, anthropic)

  ┌──────────┬────────────────────────────┬───────────┬───────┬──────────┬───────────┐
  │ Model    │ Prompt                     │ Mentioned │ Cited │ Position │ Sentiment │
  ├──────────┼────────────────────────────┼───────────┼───────┼──────────┼───────────┤
  │ openai   │ what is myproject          │ Yes       │ Yes   │ Top      │ Positive  │
  │ openai   │ best developer tool 2026   │ No        │ —     │ —        │ —         │
  │ anthropic│ myproject review           │ Yes       │ —     │ Middle   │ Positive  │
  └──────────┴────────────────────────────┴───────────┴───────┴──────────┴───────────┘

  Actionable GEO Tips:
  →  Moderate visibility. Lead every major doc section with the direct
     answer (inverted-pyramid). Publish explicit comparison pages
     (e.g. 'MyProject vs Competitor') — they rank well in LLM citations.
```

---

## Configuration

Config file: `~/.llmention/config.toml` — run `llmention config` to create it.

```toml
[providers.openai]
api_key     = "sk-..."
model       = "gpt-4o-mini"
enabled     = true
temperature = 0          # deterministic, cacheable (recommended)

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
model       = "sonar"            # sonar-pro for deeper web-grounded answers
enabled     = false
temperature = 0

# Local, free, unlimited — install: https://ollama.com
[providers.ollama]
base_url  = "http://localhost:11434"
model     = "llama3.2"
enabled   = false

# LLM-as-judge: re-evaluates each response for higher-accuracy parsing.
# Enable per-run with: --judge flag
[judge]
enabled   = false
base_url  = "http://localhost:11434"
model     = "llama3.2"

[defaults]
days        = 7    # default window for `report`
concurrency = 5    # max parallel API calls
```

---

## How It Works

1. **Prompts** — Each prompt is sent to every enabled provider concurrently (semaphore-limited).
2. **Parsing** — Detects domain mentions, link citations, position in response (Top/Middle/Bottom), and sentiment via keyword heuristics. Optionally upgrade accuracy with `--judge` (re-evaluates via a local model returning structured JSON).
3. **Cache** — Responses cached for 24 h by `SHA-256(domain|model|prompt|date)`. Re-runs are instant.
4. **Storage** — Every result saved to `~/.llmention/mentions.db` (SQLite). Records older than 90 days are pruned automatically.
5. **Report** — Colored terminal table, trend vs. previous run, and actionable GEO tips.

---

## Installation

**Cargo (recommended):**
```bash
cargo install --git https://github.com/wiramahendra/llMention
```

**From source:**
```bash
git clone https://github.com/wiramahendra/llMention
cd llmention
cargo build --release
# Binary at target/release/llmention (9.6 MB)
```

**Reduce binary size (optional):**
```bash
# Strip debug symbols
strip target/release/llmention

# Or compress with UPX (https://upx.github.io)
upx --best target/release/llmention
```

---

## Roadmap

| Phase | Feature | Status |
|-------|---------|--------|
| 1 — MVP | `audit`, `track`, `report`, `config`, `doctor` | ✅ Done |
| 2 — Generate | `llmention generate` — produce GEO-optimized markdown for any prompt | Planned |
| 3 — GUI | Thin Tauri desktop app wrapping the same SQLite store | Planned |
| 4 — Web | Optional self-hosted dashboard | Planned |

---

## Contributing

Contributions welcome. The codebase is small and modular:

```
src/
  bin/llmention.rs   CLI entrypoint (clap)
  config.rs          ~/.llmention/config.toml loader
  providers/         LlmProvider trait + OpenAI, Anthropic, xAI, Ollama
  tracker.rs         Parallel query orchestrator
  parser.rs          Rule-based mention/citation/sentiment detection
  cache.rs           24-hour file cache (SHA-256 keyed)
  storage.rs         SQLite persistence
  report.rs          Terminal output + CSV/Markdown export
  types.rs           Shared types (MentionResult, TrackSummary, …)
```

To add a new provider: implement `LlmProvider` in `src/providers/`, add a config field in `src/config.rs`, and wire it up in `tracker::build_providers`.

```bash
cargo test        # unit tests
cargo clippy      # lints
cargo build --release
```

---

## License

MIT — see [LICENSE](LICENSE).
