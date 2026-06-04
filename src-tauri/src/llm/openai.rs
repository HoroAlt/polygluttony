//! OpenAI-compatible chat-completions driver (OpenAI, Gemini OpenAI-compat,
//! Ollama, OpenRouter, …). Mirrors `llm/openai.py:OpenAiDriver`.

use async_trait::async_trait;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde_json::{json, Value};

use crate::config::Connection;
use crate::llm::error::LlmError;
use crate::llm::{base_of, get_json, parse_model_ids, post_json, timeout_of, LlmDriver};

pub struct OpenAiDriver {
    conn: Connection,
    client: reqwest::Client,
}

impl OpenAiDriver {
    pub fn new(conn: Connection) -> Self {
        Self { conn, client: reqwest::Client::new() }
    }

    fn headers(&self) -> HeaderMap {
        let mut h = HeaderMap::new();
        h.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        if let Ok(v) = HeaderValue::from_str(&format!("Bearer {}", self.conn.api_key)) {
            h.insert(AUTHORIZATION, v);
        }
        h
    }
}

#[async_trait]
impl LlmDriver for OpenAiDriver {
    async fn complete(&self, system: &str, user: &str) -> Result<String, LlmError> {
        let url = format!("{}/chat/completions", base_of(&self.conn));
        let body = json!({
            "model": self.conn.model,
            "max_tokens": self.conn.max_tokens.unwrap_or(8192),
            "messages": [
                {"role": "system", "content": system},
                {"role": "user", "content": user},
            ],
        });
        let data = post_json(&self.client, &url, self.headers(), &body, timeout_of(&self.conn)).await?;
        let text = data
            .pointer("/choices/0/message/content")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        if text.is_empty() {
            return Err(LlmError::Empty);
        }
        Ok(text)
    }

    async fn list_models(&self) -> Result<Vec<String>, LlmError> {
        let url = format!("{}/models", base_of(&self.conn));
        let data = get_json(&self.client, &url, self.headers(), timeout_of(&self.conn)).await?;
        Ok(parse_model_ids(&data))
    }

    fn model(&self) -> &str {
        &self.conn.model
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Driver;
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn conn(base: &str) -> Connection {
        Connection {
            driver: Driver::Openai, base_url: base.into(), api_key: "k".into(),
            model: "gpt-x".into(), max_tokens: Some(16), batch_dialogue_limit: None,
            timeout: Some(10), connect_timeout: None, concurrency: None,
            thinking_enabled: None, thinking_budget: None, web_search: None,
            prompt_template: None, thinking_glossary_norm_budget: None,
        }
    }

    #[tokio::test]
    async fn complete_reads_choice_content() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .and(header("authorization", "Bearer k"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "choices": [{"message": {"content": "hello"}}]
            })))
            .mount(&server)
            .await;
        let d = OpenAiDriver::new(conn(&server.uri()));
        assert_eq!(d.complete("s", "u").await.unwrap(), "hello");
    }

    #[tokio::test]
    async fn list_models_parses_ids() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/models"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": [{"id":"gpt-5.2"},{"id":"gpt-4.1"}]
            })))
            .mount(&server)
            .await;
        let models = OpenAiDriver::new(conn(&server.uri())).list_models().await.unwrap();
        assert_eq!(models, vec!["gpt-5.2", "gpt-4.1"]);
    }
}
