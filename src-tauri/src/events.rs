//! Events emitted from the Rust core to the webview during long-running
//! pipeline operations. The frontend subscribes via `onBackendEvent` (see
//! `src/lib/ipc.ts`).

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Event channel names. Keep in sync with the frontend listeners.
pub mod names {
    pub const TRANSLATION_PROGRESS: &str = "translation://progress";
    pub const GLOSSARY_PROGRESS: &str = "glossary://progress";
    pub const VERIFICATION_PROGRESS: &str = "verification://progress";
    pub const LOG: &str = "core://log";
}

/// Generic progress payload for a single unit of work (e.g. a file).
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/types/generated/")]
pub struct ProgressEvent {
    /// Identifier of the unit of work, typically the file name.
    pub id: String,
    /// Completed steps so far.
    pub completed: u32,
    /// Total steps, when known.
    pub total: Option<u32>,
    /// Optional human-readable status message.
    pub message: Option<String>,
}
