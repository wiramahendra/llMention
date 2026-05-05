# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.0] - 2026-05-06

### Changed
- Promoted v0.2 evidence engine workflow to primary CLI commands
- `init2` → `init`: Initialize project with llmention.toml
- `prompts2 discover/list` → `prompts discover/list`: Prompt management
- `audit2 run/list/show` → `audit run/list/show`: Evidence-based audits
- `report2` → `report`: Generate markdown reports from audit results
- `generate2` → `generate`: Generate content assets from audit gaps
- `diagnose2` → `diagnose`: URL crawlability diagnostics
- `compare` → `audit compare`: Compare two audit runs
- Renamed legacy commands to `audit-legacy`, `report-legacy`, `generate-legacy`

### Deprecated
- Temporary `*2` command names are deprecated but still work with warnings
- Legacy domain-based workflow is preserved as `*-legacy` commands

### Added
- Project-level configuration via `llmention.toml`
- Multi-sample audits with statistical significance
- Raw evidence storage for transparency
- Prompt categorization with intent and funnel stage
- Content gap analysis from audit results
- Before/after audit comparison
- Mock provider for testing without API keys
- Comprehensive documentation in `docs/v0.2-evidence-engine-guide.md`
- Release validation script at `scripts/validate-release.sh`

## [0.2.0] - 2026-05-05

### Added
- Initial evidence-first GEO engine implementation
- `init2` command for project initialization
- `prompts2` command for prompt discovery
- `audit2` command for multi-sample audits
- `report2` command for markdown report generation
- `generate2` command for content generation from gaps
- `compare` command for audit comparison
- `diagnose2` command for URL diagnostics
- Extended SQLite schema with audit_runs, prompts, citations tables
- Mock provider for testing

## [0.1.0] - 2026-05-01

### Added
- Initial release of LLMention
- Domain-based visibility tracking (`track`, `audit`)
- Quick audit with 12 smart prompts
- Autonomous GEO optimization agent (`optimize`)
- Content generation (`generate`)
- Project management (`projects`)
- Ollama support for fully local execution
- Plugin system for custom templates
- Desktop app (Tauri-based)

[Unreleased]: https://github.com/wiramahendra/llMention/compare/v0.3.0...HEAD
[0.3.0]: https://github.com/wiramahendra/llMention/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/wiramahendra/llMention/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/wiramahendra/llMention/releases/tag/v0.1.0
