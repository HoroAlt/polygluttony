//! Shared data types surfaced to the webview. TypeScript bindings are generated
//! by `ts-rs` into `src/types/generated/` (run `cargo test` to regenerate).

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Basic application/core metadata. Doubles as an IPC health check.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/types/generated/")]
pub struct AppInfo {
    pub name: String,
    pub version: String,
}
