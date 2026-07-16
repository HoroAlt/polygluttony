//! Configuration model. No Tauri-store, no presets persistence on the JS
//! side; we read and write a single JSON file under the user data dir.
//! Loaded via [`load`] / [`save`] in `store.rs`.

pub mod languages;
pub mod presets;
pub mod projects;
pub mod store;

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// LLM API driver.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Driver {
    /// Anthropic API / Anthropic-compatible (extended thinking).
    Anthropic,
    /// OpenAI Chat Completions / OpenRouter / local (Ollama, LM Studio).
    Openai,
    /// OpenAI Responses API (web search).
    OpenaiResponses,
}

/// A single named LLM connection.
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    /// Thinking budget for glossary extraction calls.
    #[serde(default)]
    pub thinking_glossary_budget: Option<u32>,
    /// Thinking budget for glossary normalization calls.
    #[serde(default)]
    pub thinking_glossary_norm_budget: Option<u32>,
    #[serde(default)]
    pub web_search: Option<bool>,
}

impl Connection {
    fn with_thinking_budget(&self, budget: Option<u32>) -> Connection {
        let mut c = self.clone();
        if budget.is_some() {
            c.thinking_budget = budget;
        }
        c
    }

    /// Stage clone for glossary **extraction** calls.
    pub fn for_glossary(&self) -> Connection {
        self.with_thinking_budget(self.thinking_glossary_budget)
    }

    /// Stage clone for glossary **normalization** calls.
    pub fn for_glossary_norm(&self) -> Connection {
        self.with_thinking_budget(self.thinking_glossary_norm_budget)
    }

    /// Fail-fast when thinking is enabled without a budget.
    pub fn thinking_config_error(&self) -> Option<String> {
        if self.driver == Driver::Anthropic
            && self.thinking_enabled.unwrap_or(false)
            && self.thinking_budget.is_none()
        {
            return Some(
                "thinking is enabled but no thinking budget is set — edit the connection"
                    .to_string(),
            );
        }
        None
    }

    /// Save-time validation mirroring the original UI hard-block rules.
    pub fn thinking_budget_save_error(&self) -> Option<String> {
        if self.driver != Driver::Anthropic || !self.thinking_enabled.unwrap_or(false) {
            return None;
        }
        const MIN_BUDGET: u32 = 1024;
        let budgets = [
            ("translate thinking", self.thinking_budget),
            ("glossary thinking", self.thinking_glossary_budget),
            ("normalization thinking", self.thinking_glossary_norm_budget),
        ];
        for (label, value) in budgets {
            let Some(v) = value else {
                return Some(format!("{label} budget is required when thinking is enabled"));
            };
            if v < MIN_BUDGET {
                return Some(format!("{label} budget must be at least {MIN_BUDGET} tokens"));
            }
            if let Some(max) = self.max_tokens {
                if v >= max {
                    return Some(format!("{label} budget must be less than max tokens ({max})"));
                }
            }
        }
        None
    }
}

/// Top-level persisted configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
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

impl AppConfig {
    /// Reasonable defaults; the CLI seeds `~/.config/anitranslate/config.json`
    /// from this on first run.
    pub fn defaults() -> Self {
        let mut connections = BTreeMap::new();
        connections.insert(
            "ollama".into(),
            Connection {
                driver: Driver::Openai,
                base_url: "http://localhost:11434/v1".into(),
                api_key: "ollama".into(),
                model: "qwen3.5:27b".into(),
                max_tokens: None,
                batch_dialogue_limit: None,
                timeout: Some(180),
                connect_timeout: Some(10),
                concurrency: None,
                thinking_enabled: Some(false),
                thinking_budget: None,
                thinking_glossary_budget: None,
                thinking_glossary_norm_budget: None,
                web_search: Some(false),
            },
        );
        connections.insert(
            "anthropic".into(),
            Connection {
                driver: Driver::Anthropic,
                base_url: "https://api.anthropic.com".into(),
                api_key: String::new(),
                model: "claude-sonnet-5".into(),
                max_tokens: Some(8192),
                batch_dialogue_limit: None,
                timeout: Some(120),
                connect_timeout: Some(10),
                concurrency: Some(2),
                thinking_enabled: Some(false),
                thinking_budget: None,
                thinking_glossary_budget: None,
                thinking_glossary_norm_budget: None,
                web_search: Some(false),
            },
        );
        connections.insert(
            "openai".into(),
            Connection {
                driver: Driver::Openai,
                base_url: "https://api.openai.com/v1".into(),
                api_key: String::new(),
                model: "gpt-4o-mini".into(),
                max_tokens: None,
                batch_dialogue_limit: None,
                timeout: Some(120),
                connect_timeout: Some(10),
                concurrency: Some(2),
                thinking_enabled: Some(false),
                thinking_budget: None,
                thinking_glossary_budget: None,
                thinking_glossary_norm_budget: None,
                web_search: Some(false),
            },
        );
        Self {
            default_source: "en".into(),
            default_target: "ru".into(),
            active_connection: "ollama".into(),
            personalization_model: None,
            default_workdir: None,
            connections,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn thinking_conn() -> Connection {
        Connection {
            driver: Driver::Anthropic, base_url: "https://x".into(), api_key: "k".into(),
            model: "m".into(), max_tokens: Some(16000), batch_dialogue_limit: None,
            timeout: None, connect_timeout: None, concurrency: None,
            thinking_enabled: Some(true), thinking_budget: Some(6000),
            thinking_glossary_budget: Some(12000),
            thinking_glossary_norm_budget: Some(24000), web_search: None,
        }
    }

    #[test]
    fn stage_clones_swap_the_thinking_budget() {
        let c = thinking_conn();
        assert_eq!(c.for_glossary().thinking_budget, Some(12000));
        assert_eq!(c.for_glossary_norm().thinking_budget, Some(24000));
        assert_eq!(c.for_glossary().max_tokens, Some(16000));
        assert_eq!(c.thinking_budget, Some(6000));
    }

    #[test]
    fn stage_clones_fall_back_to_translate_budget_for_legacy_configs() {
        let mut c = thinking_conn();
        c.thinking_glossary_budget = None;
        c.thinking_glossary_norm_budget = None;
        assert_eq!(c.for_glossary().thinking_budget, Some(6000));
        assert_eq!(c.for_glossary_norm().thinking_budget, Some(6000));
    }

    #[test]
    fn thinking_config_error_fires_only_when_enabled_without_budget() {
        let mut c = thinking_conn();
        assert_eq!(c.thinking_config_error(), None);
        c.thinking_budget = None;
        assert!(c.thinking_config_error().unwrap().contains("no thinking budget"));
        c.thinking_enabled = Some(false);
        assert_eq!(c.thinking_config_error(), None);
    }

    #[test]
    fn thinking_budget_save_error_enforces_ui_hard_block_rules() {
        assert!(thinking_conn()
            .thinking_budget_save_error()
            .unwrap()
            .contains("less than max tokens"));

        let mut ok = thinking_conn();
        ok.thinking_glossary_norm_budget = Some(8000);
        assert_eq!(ok.thinking_budget_save_error(), None);

        let mut c = ok.clone();
        c.thinking_budget = Some(512);
        assert!(c.thinking_budget_save_error().unwrap().contains("1024"));
    }

    #[test]
    fn glossary_budget_field_roundtrips_and_defaults_none() {
        let json = r#"{
            "driver":"anthropic","base_url":"https://x","model":"m",
            "thinking_glossary_budget":12000
        }"#;
        let c: Connection = serde_json::from_str(json).unwrap();
        assert_eq!(c.thinking_glossary_budget, Some(12000));

        let legacy: Connection =
            serde_json::from_str(r#"{"driver":"openai","base_url":"u","model":"m"}"#).unwrap();
        assert_eq!(legacy.thinking_glossary_budget, None);
    }

    #[test]
    fn config_round_trips_through_json() {
        let cfg = AppConfig::defaults();
        let s = serde_json::to_string(&cfg).unwrap();
        let back: AppConfig = serde_json::from_str(&s).unwrap();
        assert_eq!(cfg.active_connection, back.active_connection);
        assert_eq!(cfg.connections.len(), back.connections.len());
    }
}
