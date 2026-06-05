//! Anthropic Messages API driver. Mirrors `llm/anthropic.py:AnthropicDriver`.

use async_trait::async_trait;
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE};
use serde_json::{json, Value};
use std::time::Duration;

use crate::config::Connection;
use crate::llm::error::LlmError;
use crate::llm::{base_of, get_json, parse_model_ids, post_json, timeout_of, LlmDriver, LlmRequest, LlmResponse, Usage};
use crate::llm::sse;

pub struct AnthropicDriver {
    conn: Connection,
    client: reqwest::Client,
}

impl AnthropicDriver {
    pub fn new(conn: Connection) -> Self {
        let client = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(conn.connect_timeout.unwrap_or(10) as u64))
            .build()
            .unwrap_or_default();
        Self { conn, client }
    }

    fn headers(&self) -> HeaderMap {
        let mut h = HeaderMap::new();
        h.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        if let Ok(v) = HeaderValue::from_str(&self.conn.api_key) {
            h.insert("x-api-key", v);
        }
        h.insert("anthropic-version", HeaderValue::from_static("2023-06-01"));
        h
    }

    /// Build the base request body shared by `complete` and `stream`.
    fn build_body(&self, system: &str, user: &str) -> Value {
        let mut body = json!({
            "model": self.conn.model,
            "max_tokens": self.conn.max_tokens.unwrap_or(8192),
            "system": system,
            "messages": [{"role": "user", "content": user}],
        });
        if self.conn.thinking_enabled.unwrap_or(false) {
            let mut thinking = json!({"type": "enabled"});
            if let Some(b) = self.conn.thinking_budget {
                thinking["budget_tokens"] = b.into();
            }
            body["thinking"] = thinking;
        }
        body
    }
}

#[async_trait]
impl LlmDriver for AnthropicDriver {
    async fn complete(&self, system: &str, user: &str) -> Result<String, LlmError> {
        let url = format!("{}/v1/messages", base_of(&self.conn));
        let body = self.build_body(system, user);
        let data = post_json(&self.client, &url, self.headers(), &body, timeout_of(&self.conn)).await?;
        let text: String = data
            .get("content")
            .and_then(Value::as_array)
            .map(|blocks| {
                blocks
                    .iter()
                    .filter(|b| b.get("type").and_then(Value::as_str) == Some("text"))
                    .filter_map(|b| b.get("text").and_then(Value::as_str))
                    .collect::<String>()
            })
            .unwrap_or_default();
        if text.is_empty() {
            return Err(LlmError::Empty);
        }
        Ok(text)
    }

    async fn stream(&self, req: &LlmRequest) -> Result<LlmResponse, LlmError> {
        let mut body = self.build_body(&req.system, &req.user);
        body["stream"] = serde_json::Value::Bool(true);
        let mut text = String::new();
        let mut usage = Usage::default();
        sse::post_sse(
            &self.client,
            &format!("{}/v1/messages", base_of(&self.conn)),
            self.headers(),
            &body,
            timeout_of(&self.conn),
            |v| match v.get("type").and_then(|t| t.as_str()) {
                Some("content_block_delta") => {
                    if v["delta"]["type"] == "text_delta" {
                        if let Some(t) = v["delta"]["text"].as_str() {
                            text.push_str(t);
                        }
                    }
                }
                Some("message_start") => {
                    usage.input_tokens = v["message"]["usage"]["input_tokens"].as_u64();
                }
                Some("message_delta") => {
                    usage.output_tokens = v["usage"]["output_tokens"].as_u64();
                }
                _ => {}
            },
        )
        .await?;
        if text.trim().is_empty() {
            return Err(LlmError::Empty);
        }
        Ok(LlmResponse { text, usage })
    }

    async fn list_models(&self) -> Result<Vec<String>, LlmError> {
        let url = format!("{}/v1/models", base_of(&self.conn));
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
            driver: Driver::Anthropic, base_url: base.into(), api_key: "k".into(),
            model: "claude-x".into(), max_tokens: Some(16), batch_dialogue_limit: None,
            timeout: Some(10), connect_timeout: None, concurrency: None,
            thinking_enabled: Some(false), thinking_budget: None, web_search: None,
            prompt_template: None, thinking_glossary_norm_budget: None,
        }
    }

    #[tokio::test]
    async fn complete_extracts_text_blocks() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .and(header("x-api-key", "k"))
            .and(header("anthropic-version", "2023-06-01"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "content": [{"type":"text","text":"OK"},{"type":"thinking","text":"hmm"}]
            })))
            .mount(&server)
            .await;
        let d = AnthropicDriver::new(conn(&server.uri()));
        assert_eq!(d.complete("sys", "ping").await.unwrap(), "OK");
    }

    #[tokio::test]
    async fn http_error_maps_to_llm_error() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(401).set_body_string("nope"))
            .mount(&server)
            .await;
        let err = AnthropicDriver::new(conn(&server.uri()))
            .complete("s", "u").await.unwrap_err();
        assert!(err.is_auth());
    }

    #[tokio::test]
    async fn list_models_parses_ids() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v1/models"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": [{"id":"claude-opus-4-5"},{"id":"claude-haiku-4-5"}]
            })))
            .mount(&server)
            .await;
        let models = AnthropicDriver::new(conn(&server.uri())).list_models().await.unwrap();
        assert_eq!(models, vec!["claude-opus-4-5", "claude-haiku-4-5"]);
    }

    #[tokio::test]
    async fn stream_accumulates_text_deltas() {
        let server = MockServer::start().await;
        let sse_body = concat!(
            "event: message_start\n",
            "data: {\"type\":\"message_start\",\"message\":{\"usage\":{\"input_tokens\":10}}}\n\n",
            "event: content_block_delta\n",
            "data: {\"type\":\"content_block_delta\",\"delta\":{\"type\":\"text_delta\",\"text\":\"Hel\"}}\n\n",
            "event: content_block_delta\n",
            "data: {\"type\":\"content_block_delta\",\"delta\":{\"type\":\"thinking_delta\",\"thinking\":\"hmm\"}}\n\n",
            "event: content_block_delta\n",
            "data: {\"type\":\"content_block_delta\",\"delta\":{\"type\":\"text_delta\",\"text\":\"lo\"}}\n\n",
            "event: message_delta\n",
            "data: {\"type\":\"message_delta\",\"usage\":{\"output_tokens\":5}}\n\n",
        );
        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "text/event-stream")
                    .set_body_raw(sse_body, "text/event-stream"),
            )
            .mount(&server)
            .await;
        let driver = AnthropicDriver::new(conn(&server.uri()));
        let resp = driver
            .stream(&LlmRequest { system: "s".into(), user: "u".into() })
            .await
            .unwrap();
        assert_eq!(resp.text, "Hello");
        assert_eq!(resp.usage.input_tokens, Some(10));
        assert_eq!(resp.usage.output_tokens, Some(5));
    }
}
