# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [0.1.0] — 2026-07-16

### Added

- Rust workspace split into `crates/core` (engine) and
  `crates/bin/anitranslate` (CLI).
- Tauri-free port of the polygluttony translation engine:
  - ASS tag-preserving parser and writer
    (`crates/core/src/ass/{decode,parse,tags,writer}.rs`).
  - 6-category glossary pipeline with world-type detection
    (`crates/core/src/glossary/{build,model,personalize,normalize}.rs`).
  - Multi-provider LLM driver layer (Anthropic, OpenAI Chat Completions,
    OpenAI Responses, OpenRouter, Ollama) with AIMD-bounded concurrency,
    Retry-After honoring, soft-404 gateway detection
    (`crates/core/src/llm/`).
  - Translation pipeline with batch halving, prefix salvage, drift
    detection, scoped retranslation, MAX_RETRANSLATION_ATTEMPTS cap
    (`crates/core/src/translation/pipeline.rs`).
- CLI subcommands: `translate`, `build-glossary`, `inspect`, `config`.
- Default seeded config with three ready-to-use connections
  (`ollama`, `anthropic`, `openai`).
- Localhost-only safety check for keyless connections.
- 255 unit tests passing on the engine.

### Changed

- Polygluttony's Tauri shell, React UI, TypeScript bindings, and Vite
  build pipeline: removed.
- `ts_rs` TypeScript binding generation: removed.
- `tauri-plugin-{store,fs,notification,dialog,opener}`: removed.
- `tracing`/`tracing-subscriber`: replaced with `tracing` only.
- `rand` 0.8 → 0.9 (matches upstream API used in the source).
- `reqwest` features: `rustls` → `rustls-tls`.

### Security

- `unsafe_code = "forbid"` workspace lint.
- No `eval`, no `child_process`, no `Command::new`, no `tauri::Manager`.
- All `process::Command` references removed (the engine had zero).
- Ollama connections limited to `localhost` / `127.0.0.1` / `[::1]` for
  keyless operation. Custom URLs require a non-empty API key.
- Co-authored-by trailer on AI-assisted commits.

[0.1.0]: https://github.com/HoroAlt/anitranslate/releases/tag/v0.1.0
