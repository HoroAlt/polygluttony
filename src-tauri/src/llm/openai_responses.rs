//! OpenAI Responses API driver (web-search capable). Mirrors
//! `llm/openai_responses.py:OpenAiResponsesDriver`.

use async_trait::async_trait;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde_json::{json, Value};

use crate::config::Connection;
use crate::llm::error::LlmError;
use crate::llm::{base_of, get_json, parse_model_ids, post_json, timeout_of, LlmDriver};

pub struct OpenAiResponsesDriver {
    conn: Connection,
    client: reqwest::Client,
}

impl OpenAiResponsesDriver {
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
impl LlmDriver for OpenAiResponsesDriver {
    async fn complete(&self, system: &str, user: &str) -> Result<String, LlmError> {
        let url = format!("{}/responses", base_of(&self.conn));
        let mut body = json!({
            "model": self.conn.model,
            "max_output_tokens": self.conn.max_tokens.unwrap_or(8192),
            "input": [
                {"role": "system", "content": system},
                {"role": "user", "content": user},
            ],
        });
        if self.conn.web_search.unwrap_or(false) {
            body["tools"] = json!([{"type": "web_search_preview"}]);
        }
        let data = post_json(&self.client, &url, self.headers(), &body, timeout_of(&self.conn)).await?;
        let text: String = data
            .get("output")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter(|it| it.get("type").and_then(Value::as_str) == Some("message"))
                    .filter_map(|it| it.get("content").and_then(Value::as_array))
                    .flatten()
                    .filter(|b| b.get("type").and_then(Value::as_str) == Some("output_text"))
                    .filter_map(|b| b.get("text").and_then(Value::as_str))
                    .collect::<String>()
            })
            .unwrap_or_default();
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
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn conn(base: &str) -> Connection {
        Connection {
            driver: Driver::OpenaiResponses, base_url: base.into(), api_key: "k".into(),
            model: "gpt-5.2".into(), max_tokens: Some(16), batch_dialogue_limit: None,
            timeout: Some(10), connect_timeout: None, concurrency: None,
            thinking_enabled: None, thinking_budget: None, web_search: Some(false),
            prompt_template: None, thinking_glossary_norm_budget: None,
        }
    }

    #[tokio::test]
    async fn complete_reads_output_text() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/responses"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "output": [
                    {"type":"reasoning","content":[]},
                    {"type":"message","content":[{"type":"output_text","text":"hi"}]}
                ]
            })))
            .mount(&server)
            .await;
        let d = OpenAiResponsesDriver::new(conn(&server.uri()));
        assert_eq!(d.complete("s", "u").await.unwrap(), "hi");
    }
}
