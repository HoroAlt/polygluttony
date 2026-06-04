//! Custom-connection API-format detection (spec §5.7). Probes both wire formats
//! and disambiguates by HTTP status: any non-404 response means the route
//! exists (so a wrong key — 401/403 — still reveals the format).

use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde_json::json;

use crate::config::Driver;
use crate::llm::error::LlmError;

enum Probe {
    Exists,
    NotHere,
    Unreachable,
}

async fn probe(client: &reqwest::Client, url: &str, headers: HeaderMap, body: serde_json::Value) -> Probe {
    match client
        .post(url)
        .headers(headers)
        .json(&body)
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
    {
        Ok(resp) => {
            if resp.status().as_u16() == 404 {
                Probe::NotHere
            } else {
                Probe::Exists
            }
        }
        Err(_) => Probe::Unreachable,
    }
}

/// Determine whether a base URL speaks OpenAI- or Anthropic-style.
pub async fn detect_format(base_url: &str, api_key: &str) -> Result<Driver, LlmError> {
    let client = reqwest::Client::new();
    let base = base_url.trim_end_matches('/');

    // Probe 1: OpenAI chat completions.
    let mut oai = HeaderMap::new();
    oai.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    if let Ok(v) = HeaderValue::from_str(&format!("Bearer {api_key}")) {
        oai.insert(AUTHORIZATION, v);
    }
    let oai_body = json!({"model":"probe","max_tokens":1,
        "messages":[{"role":"user","content":"ping"}]});
    if let Probe::Exists =
        probe(&client, &format!("{base}/chat/completions"), oai, oai_body).await
    {
        return Ok(Driver::Openai);
    }

    // Probe 2: Anthropic messages.
    let mut ant = HeaderMap::new();
    ant.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    if let Ok(v) = HeaderValue::from_str(api_key) {
        ant.insert("x-api-key", v);
    }
    ant.insert("anthropic-version", HeaderValue::from_static("2023-06-01"));
    let ant_body = json!({"model":"probe","max_tokens":1,
        "messages":[{"role":"user","content":"ping"}]});
    if let Probe::Exists =
        probe(&client, &format!("{base}/v1/messages"), ant, ant_body).await
    {
        return Ok(Driver::Anthropic);
    }

    Err(LlmError::Transport(
        "couldn't determine the API format at this URL".into(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Driver;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn detects_openai_when_chat_route_exists() {
        let server = MockServer::start().await;
        Mock::given(method("POST")).and(path("/chat/completions"))
            .respond_with(ResponseTemplate::new(401)) // exists but bad key
            .mount(&server).await;
        // No /v1/messages mounted -> wiremock returns 404 for it.
        let d = detect_format(&server.uri(), "k").await.unwrap();
        assert_eq!(d, Driver::Openai);
    }

    #[tokio::test]
    async fn detects_anthropic_when_only_messages_route_exists() {
        let server = MockServer::start().await;
        Mock::given(method("POST")).and(path("/v1/messages"))
            .respond_with(ResponseTemplate::new(400))
            .mount(&server).await;
        let d = detect_format(&server.uri(), "k").await.unwrap();
        assert_eq!(d, Driver::Anthropic);
    }

    #[tokio::test]
    async fn undetermined_when_both_404() {
        let server = MockServer::start().await; // nothing mounted -> all 404
        assert!(detect_format(&server.uri(), "k").await.is_err());
    }
}
