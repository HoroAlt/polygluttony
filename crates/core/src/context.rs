//! Tauri-free app context.
//!
//! The original polygluttony coupled its run state to `tauri::AppHandle` because
//! the GUI was the only consumer. We extract a small trait so the engine can
//! run headless from a CLI; Tauri-specific plumbing stays out of this crate.

use std::path::{Path, PathBuf};

/// Minimal emit primitive.
pub trait EventSink: Send + Sync {
    fn emit(&self, channel: &str, payload: serde_json::Value);
}

/// Resolve the on-disk path to the user-data directory.
pub trait AppContext: Send + Sync {
    fn app_data_dir(&self) -> PathBuf;
    fn emit(&self, channel: &str, payload: serde_json::Value);
}

impl<T: AppContext> EventSink for T {
    fn emit(&self, channel: &str, payload: serde_json::Value) {
        AppContext::emit(self, channel, payload);
    }
}

/// CLI implementation: holds a data dir and a callback that receives
/// `(channel, &json_value)`. The CLI runner plugs in `tracing::info!`,
/// `println!`, or a custom JSONL renderer.
pub struct CliContext {
    pub data_dir: PathBuf,
    pub on_event: Box<dyn Fn(&str, &serde_json::Value) + Send + Sync>,
}

impl std::fmt::Debug for CliContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CliContext")
            .field("data_dir", &self.data_dir)
            .finish_non_exhaustive()
    }
}

impl CliContext {
    pub fn new(data_dir: impl Into<PathBuf>) -> Self {
        Self {
            data_dir: data_dir.into(),
            on_event: Box::new(|_, _| {}),
        }
    }

    pub fn with_status<F>(mut self, f: F) -> Self
    where
        F: Fn(&str, &serde_json::Value) + Send + Sync + 'static,
    {
        self.on_event = Box::new(f);
        self
    }

    /// Ensure the data dir exists.
    pub fn ensure_dirs(&self) -> std::io::Result<()> {
        std::fs::create_dir_all(self.data_dir.join("prompts"))?;
        Ok(())
    }
}

impl AppContext for CliContext {
    fn app_data_dir(&self) -> PathBuf {
        self.data_dir.clone()
    }

    fn emit(&self, channel: &str, payload: serde_json::Value) {
        (self.on_event)(channel, &payload);
    }
}

/// Resolve the prompts/overrides directory under the data dir and ensure it
/// exists.
pub fn overrides_dir(data_dir: &Path) -> std::io::Result<PathBuf> {
    let p = data_dir.join("prompts").join("overrides");
    std::fs::create_dir_all(&p)?;
    Ok(p)
}
