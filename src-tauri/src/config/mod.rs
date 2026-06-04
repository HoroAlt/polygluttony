//! Application configuration model. Persisted on the frontend via the Tauri
//! store plugin; these types define the schema and generate matching TS bindings.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// LLM API driver. Mirrors the original config's `driver` field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
#[ts(export, export_to = "../../src/types/generated/")]
pub enum Driver {
    /// Anthropic API / Anthropic-compatible (extended thinking).
    Anthropic,
    /// OpenAI Chat Completions / OpenRouter / local (Ollama, LM Studio).
    Openai,
    /// OpenAI Responses API (web search).
    OpenaiResponses,
}

/// A single named LLM connection.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/types/generated/")]
pub struct Connection {
    pub driver: Driver,
    pub base_url: String,
    #[serde(default)]
    pub api_key: String,
    pub model: String,
    #[serde(default)]
    pub max_tokens: Option<u32>,
    #[serde(default)]
    pub batch_dialogue_limit: Option<u32>,
    #[serde(default)]
    pub timeout: Option<u32>,
    #[serde(default)]
    pub connect_timeout: Option<u32>,
    #[serde(default)]
    pub concurrency: Option<u32>,
    #[serde(default)]
    pub thinking_enabled: Option<bool>,
    #[serde(default)]
    pub thinking_budget: Option<u32>,
    #[serde(default)]
    pub web_search: Option<bool>,
}

/// Top-level persisted configuration.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/types/generated/")]
pub struct AppConfig {
    pub default_source: String,
    pub default_target: String,
    pub active_connection: String,
    #[serde(default)]
    pub personalization_model: Option<String>,
    #[serde(default)]
    pub default_workdir: Option<String>,
    pub connections: BTreeMap<String, Connection>,
}
