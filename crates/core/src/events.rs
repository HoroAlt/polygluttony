//! Plain data enums for the translation pipeline.
//!
//! Consumers: the CLI runner, which logs events to stdout; future UIs would
//! re-serialize them as JSON. Mirrors the original `events.rs` minus the
//! `ts_rs` TypeScript bindings (we have no TS host).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FileStateKind {
    Pending,
    Translating,
    Retranslating,
    Cleanup,
    Verifying,
    Done,
    Warning,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Debug,
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogPhase {
    Parse,
    Batch,
    Cleanup,
    Verify,
    Llm,
    Error,
    Retranslate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyIssue {
    pub line_id: u32,
    pub source: String,
    pub translation: String,
    pub issue_type: String,
    pub description: String,
    pub severity: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileResult {
    /// File name relative to the run folder; never an absolute path.
    pub file: String,
    pub success: bool,
    pub total_lines: u32,
    pub translated_lines: u32,
    pub has_warnings: bool,
    pub issues: Vec<VerifyIssue>,
    pub output_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RunEvent {
    State {
        file: String,
        state: FileStateKind,
        detail: Option<String>,
    },
    Progress {
        file: String,
        translated: u32,
        total: u32,
        batch: u32,
        total_batches: u32,
        retries: u32,
    },
    Log {
        file: Option<String>,
        level: LogLevel,
        phase: LogPhase,
        message: String,
    },
    FileDone {
        file: String,
        has_warnings: bool,
        issues: Vec<VerifyIssue>,
    },
    Error {
        file: String,
        message: String,
    },
    RunFinished {
        results: Vec<FileResult>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GlossaryPhase {
    Loading,
    Reference,
    Extracting,
    Normalizing,
    Personalizing,
    Saving,
}

pub use crate::glossary::diff::{CategoryDiff, DiffStatus, GlossaryDiff, TermDiff};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlossaryBuildSummary {
    pub world_type: String,
    pub files_processed: u32,
    pub batches_processed: u32,
    pub batches_total: u32,
    pub terms_extracted: u32,
    pub terms_final: u32,
    pub normalized: bool,
    pub personalized: bool,
    pub aborted: bool,
    pub cancelled: bool,
    pub errors: Vec<String>,
    pub diff: GlossaryDiff,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TermHit {
    pub category: String,
    pub source: String,
    pub target: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum GlossaryEvent {
    Phase {
        phase: GlossaryPhase,
        detail: Option<String>,
    },
    Progress {
        done: u32,
        total: u32,
    },
    Terms {
        batch: u32,
        hits: Vec<TermHit>,
    },
    Log {
        level: LogLevel,
        message: String,
    },
    Done {
        summary: GlossaryBuildSummary,
    },
    Error {
        message: String,
    },
    FileChanged,
}

/// Event channel names. Kept in sync with the original Tauri constants.
pub const TRANSLATION_EVENT: &str = "translation://event";
pub const GLOSSARY_EVENT: &str = "glossary://event";
