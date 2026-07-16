//! anitranslate-core
//!
//! Translation engine for `.ass`/`.srt` subtitle files using LLMs.
//!
//! Pipeline (port of polygluttony/blyat-uk — Tauri-free, CLI-friendly):
//!
//! 1. `ass::decode` parses the source file, `ass::parse` extracts dialogues,
//!    `ass::tags::strip_for_text` peels overlay tags off each line.
//! 2. `translation::pipeline` runs the batched LLM translation loop with
//!    line-marker alignment, prefix salvage, batch-halving, cleanup,
//!    drift detection, verify, and scoped retranslation.
//! 3. `glossary` builds, normalizes, and persists per-folder glossaries;
//!    the translate pipeline uses them as part of each prompt.
//! 4. `llm` is the OpenAI / Anthropic / Ollama driver layer with
//!    AIMD-bounded concurrency, SSE parsing, soft-404 detection, and
//!    transient-error retry.
//! 5. `validation` is the line-marker, alignment, drift, and scope
//!    machinery that protects translation from LLM slip-ups.
//!
//! The CLI in `crates/bin/anitranslate` owns the user-facing surface: it
//! loads/saves `AppConfig` via `config::store`, wires `LlmService` to a
//! `translation::pipeline::FileJob`, and renders `RunEvent`s to stdout.

#![warn(unused_must_use)]

pub mod ass;
pub mod config;
pub mod context;
pub mod error;
pub mod events;
pub mod glossary;
pub mod llm;
pub mod models;
pub mod prompts;
pub mod translation;
pub mod utils;
pub mod validation;

pub use error::{AppError, AppResult};
