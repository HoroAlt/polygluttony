# Step 1 — App Shell + Connections Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the new single-window shell (icon rail + header + status bar) and a fully working **Connections** view, backed by a real Rust port of the LLM driver layer so the **Test** button hits real providers; persist config via the Tauri store.

**Architecture:** Rust engine inside `src-tauri/src/` exposes `#[tauri::command]`s for connection CRUD, `test_connection`, `list_models`, and `first_run_status`. The LLM layer is an `async_trait LlmDriver` with three drivers (anthropic / openai / openai-responses), a status-code-based format detector for Custom, and `LlmError`. The React frontend (TanStack Router) renders the rail + Connections view, themed with the `theme.py` token palette, talking to Rust through ts-rs-typed IPC wrappers.

**Tech Stack:** Rust (tauri 2, reqwest, async-trait, serde_json, thiserror, ts-rs; wiremock for tests) + React 19 / TS (TanStack Router+Query, Zustand, react-hook-form+zod, cmdk, @phosphor-icons/react, Tailwind v4 + shadcn/maia).

**Reference docs (read before starting):**
- Spec: `docs/superpowers/specs/2026-06-04-step1-shell-and-connections-design.md` (authoritative).
- UI: `../polygluttony-docs/windows/00-shell-rail-statusbar.md`, `windows/02-connections.md`, `01-design-system.md`, `03-operations-and-flows.md`.
- Python source of truth: `../subs-translate/subs_translator/config/settings.py`, `llm/anthropic.py`, `llm/openai.py`, `llm/openai_responses.py`, `llm/client.py`.

**Conventions:**
- Backend tasks are strict TDD (write failing test → run → implement → pass → commit).
- Frontend tasks have **no test runner configured**; their verification gate is `bun run build` (route-gen → `tsc` → vite) plus, where noted, manual smoke via `bun tauri dev`. Still commit per task.
- Run backend tests with `cargo test --manifest-path src-tauri/Cargo.toml`.
- After changing any `#[derive(TS)]` type, regenerate bindings (Task 11) and re-run `bun run build`.
- Commit messages: `feat:` / `test:` / `chore:` / `refactor:` as appropriate.

---

## File structure (created / modified)

**Backend (`src-tauri/`):**
- `Cargo.toml` — *modify*: add `async-trait`; dev-dep `wiremock`.
- `src/config/mod.rs` — *modify*: extend `Connection`; re-export presets/store.
- `src/config/presets.rs` — *create*: preset table, default `AppConfig`, curated model lists, `Preset` type.
- `src/config/store.rs` — *create*: pure `AppConfig` mutation helpers + thin Tauri-store glue.
- `src/llm/mod.rs` — *modify*: `LlmDriver` trait, `create_driver`, module wiring, shared `post_json` helper.
- `src/llm/error.rs` — *create*: `LlmError` + `is_retryable`/`is_auth`.
- `src/llm/anthropic.rs` — *create*: `AnthropicDriver`.
- `src/llm/openai.rs` — *create*: `OpenAiDriver`.
- `src/llm/openai_responses.rs` — *create*: `OpenAiResponsesDriver`.
- `src/llm/detect.rs` — *create*: `detect_format`.
- `src/models/mod.rs` — *modify*: add IPC DTOs (`ConnectionsView`, `ConnectionSummary`, `TestResult`, `FirstRunStatus`).
- `src/commands/mod.rs` — *modify*: re-export; keep `app_info`.
- `src/commands/connections.rs` — *create*: connection commands + `test_connection` + `list_models` + `first_run_status` + `list_presets`.
- `src/lib.rs` — *modify*: register commands; manage `reqwest::Client` if useful.

**Frontend (`src/`):**
- `index.css` — *modify*: `theme.py` token palette.
- `package.json` — *modify*: add `@phosphor-icons/react`; `gen:bindings` + `gen:routes` scripts.
- `components/nav-rail.tsx` — *create*.
- `components/app-layout.tsx` — *modify*: rail + header + main + status bar grid.
- `components/status-bar.tsx`, `components/page-header.tsx` — *modify*: reworked.
- `components/status-chip.tsx`, `state-chip.tsx`, `help-text.tsx`, `setup-field.tsx`, `empty-state.tsx`, `section-help.tsx` — *create*.
- `lib/ipc.ts` — *modify*: typed command wrappers.
- `stores/app-store.ts` — *modify*: connection/first-run UI state.
- `routes/index.tsx` — *modify*: first-run redirect + placeholder home.
- `routes/connections.tsx` — *create*; `routes/settings.tsx` — *modify*; `routes/help.tsx` — *create*; `routes/{glossary,translate,verify,project}.tsx` — *create/modify* as gated placeholders.
- `features/connections/` — *create*: `connections-page.tsx`, `connection-list.tsx`, `connection-editor.tsx`, `model-combobox.tsx`, `use-connections.ts`.

---

## Task 0: Project setup (git, deps, gitignore)

**Files:**
- Modify: `.gitignore`
- Modify: `src-tauri/Cargo.toml`

- [ ] **Step 1: Initialize git and ignore Rust build output**

Append to `.gitignore` (only if not already present):

```gitignore

# Rust / Tauri build output
/src-tauri/target
/src-tauri/gen/schemas
```

Then:

```bash
git init
git add -A
git commit -m "chore: initial scaffold snapshot before step 1"
```

- [ ] **Step 2: Add Rust deps**

In `src-tauri/Cargo.toml`, under `[dependencies]` add:

```toml
async-trait = "0.1"
```

Add a dev-dependencies section at the end of the file:

```toml
[dev-dependencies]
wiremock = "0.6"
```

- [ ] **Step 3: Verify it builds**

Run: `cargo build --manifest-path src-tauri/Cargo.toml`
Expected: compiles (downloads async-trait/wiremock).

- [ ] **Step 4: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/Cargo.lock
git commit -m "chore: add async-trait and wiremock deps"
```

---

## Task 1: Extend the Connection config model

**Files:**
- Modify: `src-tauri/src/config/mod.rs`
- Test: inline `#[cfg(test)]` in `src-tauri/src/config/mod.rs`

- [ ] **Step 1: Write the failing test**

Add at the bottom of `config/mod.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connection_round_trips_new_fields() {
        let json = r#"{
            "driver":"anthropic","base_url":"https://x","api_key":"k","model":"m",
            "prompt_template":"qwen","thinking_glossary_norm_budget":4096
        }"#;
        let c: Connection = serde_json::from_str(json).unwrap();
        assert_eq!(c.prompt_template.as_deref(), Some("qwen"));
        assert_eq!(c.thinking_glossary_norm_budget, Some(4096));
        // Optional fields default to None when absent.
        let minimal: Connection =
            serde_json::from_str(r#"{"driver":"openai","base_url":"u","model":"m"}"#).unwrap();
        assert_eq!(minimal.prompt_template, None);
        assert_eq!(minimal.api_key, "");
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml connection_round_trips_new_fields`
Expected: FAIL — `no field prompt_template` / `thinking_glossary_norm_budget`.

- [ ] **Step 3: Add the two fields**

In `config/mod.rs`, inside `struct Connection`, after `web_search`:

```rust
    #[serde(default)]
    pub prompt_template: Option<String>,
    #[serde(default)]
    pub thinking_glossary_norm_budget: Option<u32>,
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --manifest-path src-tauri/Cargo.toml connection_round_trips_new_fields`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/config/mod.rs
git commit -m "feat(config): add prompt_template + thinking_glossary_norm_budget to Connection"
```

---

## Task 2: Presets, default config, and curated model lists

**Files:**
- Create: `src-tauri/src/config/presets.rs`
- Modify: `src-tauri/src/config/mod.rs` (add `pub mod presets;` and re-exports)
- Test: inline `#[cfg(test)]` in `presets.rs`

- [ ] **Step 1: Wire the module**

In `config/mod.rs` top, add: `pub mod presets;`

- [ ] **Step 2: Write the failing test**

Create `config/presets.rs` with only the tests first:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Driver;

    #[test]
    fn default_config_has_seeded_connections() {
        let cfg = default_config();
        assert_eq!(cfg.default_source, "zh");
        assert_eq!(cfg.default_target, "en");
        assert_eq!(cfg.active_connection, "anthropic");
        assert_eq!(cfg.personalization_model.as_deref(), Some("openai"));
        for name in ["anthropic", "google", "openai", "ollama"] {
            assert!(cfg.connections.contains_key(name), "missing seed {name}");
        }
        // Cloud seeds ship without a key; ollama carries a placeholder.
        assert_eq!(cfg.connections["anthropic"].api_key, "");
        assert_eq!(cfg.connections["ollama"].api_key, "ollama");
    }

    #[test]
    fn presets_cover_five_providers_with_drivers() {
        let presets = presets();
        let keys: Vec<&str> = presets.iter().map(|p| p.key.as_str()).collect();
        assert_eq!(keys, ["anthropic", "google", "openai", "ollama", "custom"]);
        assert_eq!(presets[0].driver, Some(Driver::Anthropic));   // anthropic
        assert_eq!(presets[1].driver, Some(Driver::Openai));      // google
        assert_eq!(presets[2].driver, Some(Driver::OpenaiResponses)); // openai
        assert_eq!(presets[3].driver, Some(Driver::Openai));      // ollama
        assert_eq!(presets[4].driver, None);                      // custom = auto-detect
        assert!(presets[0].models.iter().any(|m| m.starts_with("claude")));
    }
}
```

- [ ] **Step 3: Run test to verify it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --lib presets`
Expected: FAIL — `default_config`/`presets`/`Preset` not found.

- [ ] **Step 4: Implement presets + defaults**

Prepend to `presets.rs` (above the tests):

```rust
//! Provider presets, curated model lists, and the seeded default config.
//! Ports `config/settings.py:get_default_config` adapted to the 5-provider list.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::config::{AppConfig, Connection, Driver};

/// A provider preset shown in the Connections "Provider" dropdown.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/types/generated/")]
pub struct Preset {
    pub key: String,
    pub label: String,
    /// `None` = auto-detect on Test (Custom).
    pub driver: Option<Driver>,
    pub base_url: String,
    pub model: String,
    /// Curated model suggestions (live `/models` fetch supplements these).
    pub models: Vec<String>,
}

fn conn(driver: Driver, base_url: &str, model: &str, api_key: &str) -> Connection {
    Connection {
        driver,
        base_url: base_url.to_string(),
        api_key: api_key.to_string(),
        model: model.to_string(),
        max_tokens: Some(16000),
        batch_dialogue_limit: Some(100),
        timeout: Some(120),
        connect_timeout: Some(10),
        concurrency: Some(5),
        thinking_enabled: None,
        thinking_budget: None,
        web_search: None,
        prompt_template: None,
        thinking_glossary_norm_budget: None,
    }
}

const ANTHROPIC_MODELS: &[&str] =
    &["claude-opus-4-5", "claude-sonnet-4-5", "claude-haiku-4-5"];
const OPENAI_MODELS: &[&str] = &["gpt-5.2", "gpt-5.1", "gpt-5", "gpt-4.1", "o4-mini"];
const GOOGLE_MODELS: &[&str] = &["gemini-2.5-pro", "gemini-2.5-flash", "gemini-2.0-flash"];
const OLLAMA_MODELS: &[&str] = &["llama3.1", "qwen2.5"];

fn models(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| s.to_string()).collect()
}

const ANTHROPIC_BASE: &str = "https://api.anthropic.com";
const GOOGLE_BASE: &str = "https://generativelanguage.googleapis.com/v1beta/openai/";
const OPENAI_BASE: &str = "https://api.openai.com/v1";
const OLLAMA_BASE: &str = "http://localhost:11434/v1";

/// The provider preset table for the UI dropdown.
pub fn presets() -> Vec<Preset> {
    vec![
        Preset { key: "anthropic".into(), label: "Anthropic".into(),
            driver: Some(Driver::Anthropic), base_url: ANTHROPIC_BASE.into(),
            model: "claude-opus-4-5".into(), models: models(ANTHROPIC_MODELS) },
        Preset { key: "google".into(), label: "Google (Gemini)".into(),
            driver: Some(Driver::Openai), base_url: GOOGLE_BASE.into(),
            model: "gemini-2.5-pro".into(), models: models(GOOGLE_MODELS) },
        Preset { key: "openai".into(), label: "OpenAI".into(),
            driver: Some(Driver::OpenaiResponses), base_url: OPENAI_BASE.into(),
            model: "gpt-5.2".into(), models: models(OPENAI_MODELS) },
        Preset { key: "ollama".into(), label: "Ollama (local)".into(),
            driver: Some(Driver::Openai), base_url: OLLAMA_BASE.into(),
            model: "llama3.1".into(), models: models(OLLAMA_MODELS) },
        Preset { key: "custom".into(), label: "Custom".into(),
            driver: None, base_url: String::new(),
            model: String::new(), models: vec![] },
    ]
}

/// The default `AppConfig` seeded on first run.
pub fn default_config() -> AppConfig {
    let mut connections: BTreeMap<String, Connection> = BTreeMap::new();
    connections.insert("anthropic".into(),
        conn(Driver::Anthropic, ANTHROPIC_BASE, "claude-opus-4-5", ""));
    connections.insert("google".into(),
        conn(Driver::Openai, GOOGLE_BASE, "gemini-2.5-pro", ""));
    connections.insert("openai".into(),
        conn(Driver::OpenaiResponses, OPENAI_BASE, "gpt-5.2", ""));
    connections.insert("ollama".into(),
        conn(Driver::Openai, OLLAMA_BASE, "llama3.1", "ollama"));

    AppConfig {
        default_source: "zh".into(),
        default_target: "en".into(),
        active_connection: "anthropic".into(),
        personalization_model: Some("openai".into()),
        default_workdir: None,
        connections,
    }
}
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --lib presets`
Expected: PASS (both tests).

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/config/mod.rs src-tauri/src/config/presets.rs
git commit -m "feat(config): provider presets, curated models, seeded default config"
```

---

## Task 3: Pure AppConfig mutation helpers + Tauri store glue

**Files:**
- Create: `src-tauri/src/config/store.rs`
- Modify: `src-tauri/src/config/mod.rs` (add `pub mod store;`)
- Test: inline `#[cfg(test)]` in `store.rs` (pure helpers only)

> Rationale: the Tauri `Store` needs a live `AppHandle`, so it is awkward to unit test.
> Keep all logic in **pure functions over `AppConfig`** (tested here); the Tauri glue is a
> thin, untested adapter that calls them.

- [ ] **Step 1: Wire the module**

In `config/mod.rs`: add `pub mod store;`

- [ ] **Step 2: Write the failing tests**

Create `config/store.rs` with tests first:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{presets::default_config, Connection, Driver};

    fn sample() -> Connection {
        Connection {
            driver: Driver::Openai, base_url: "u".into(), api_key: "k".into(),
            model: "m".into(), max_tokens: None, batch_dialogue_limit: None,
            timeout: None, connect_timeout: None, concurrency: None,
            thinking_enabled: None, thinking_budget: None, web_search: None,
            prompt_template: None, thinking_glossary_norm_budget: None,
        }
    }

    #[test]
    fn upsert_then_read_back() {
        let mut cfg = default_config();
        upsert_connection(&mut cfg, "mine", sample());
        assert_eq!(cfg.connections["mine"].api_key, "k");
    }

    #[test]
    fn set_active_requires_existing() {
        let mut cfg = default_config();
        assert!(set_active(&mut cfg, "anthropic").is_ok());
        assert_eq!(cfg.active_connection, "anthropic");
        assert!(set_active(&mut cfg, "nope").is_err());
    }

    #[test]
    fn delete_blocks_removing_active() {
        let mut cfg = default_config();
        set_active(&mut cfg, "anthropic").unwrap();
        // Removing the active connection is refused.
        assert!(remove_connection(&mut cfg, "anthropic").is_err());
        // A non-active one is removable.
        assert!(remove_connection(&mut cfg, "google").is_ok());
        assert!(!cfg.connections.contains_key("google"));
    }

    #[test]
    fn first_run_is_true_when_no_key() {
        let cfg = default_config(); // only ollama has a placeholder key
        // ollama's placeholder counts as a "usable" key, so default is NOT first-run.
        assert!(has_usable_connection(&cfg));
        let mut empty = default_config();
        for c in empty.connections.values_mut() { c.api_key.clear(); }
        assert!(!has_usable_connection(&empty));
    }
}
```

- [ ] **Step 3: Run tests to verify they fail**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --lib store`
Expected: FAIL — helper fns not defined.

- [ ] **Step 4: Implement the pure helpers + thin glue**

Prepend to `store.rs` (above tests):

```rust
//! Config persistence. Pure `AppConfig` helpers (unit-tested) + a thin Tauri
//! store adapter (`load`/`save`) that seeds defaults on first run.

use tauri::{AppHandle, Runtime};
use tauri_plugin_store::StoreExt;

use crate::config::{presets::default_config, AppConfig, Connection};
use crate::error::{AppError, AppResult};

const STORE_FILE: &str = "config.json";
const STORE_KEY: &str = "app";

// ---- pure helpers over AppConfig -------------------------------------------

pub fn upsert_connection(cfg: &mut AppConfig, name: &str, conn: Connection) {
    cfg.connections.insert(name.to_string(), conn);
}

pub fn set_active(cfg: &mut AppConfig, name: &str) -> AppResult<()> {
    if !cfg.connections.contains_key(name) {
        return Err(AppError::Other(format!("unknown connection: {name}")));
    }
    cfg.active_connection = name.to_string();
    Ok(())
}

pub fn set_personalization(cfg: &mut AppConfig, name: &str) -> AppResult<()> {
    if !cfg.connections.contains_key(name) {
        return Err(AppError::Other(format!("unknown connection: {name}")));
    }
    cfg.personalization_model = Some(name.to_string());
    Ok(())
}

pub fn remove_connection(cfg: &mut AppConfig, name: &str) -> AppResult<()> {
    if cfg.active_connection == name {
        return Err(AppError::Other(
            "reassign the active connection before removing it".into(),
        ));
    }
    cfg.connections.remove(name);
    Ok(())
}

/// First-run check (O21): any connection carrying a non-empty api_key.
pub fn has_usable_connection(cfg: &AppConfig) -> bool {
    cfg.connections.values().any(|c| !c.api_key.trim().is_empty())
}

// ---- Tauri store adapter (thin; not unit-tested) ---------------------------

/// Load the config from the store, seeding + persisting defaults on first run.
pub fn load<R: Runtime>(app: &AppHandle<R>) -> AppResult<AppConfig> {
    let store = app.store(STORE_FILE).map_err(|e| AppError::Other(e.to_string()))?;
    match store.get(STORE_KEY) {
        Some(value) => serde_json::from_value(value).map_err(AppError::from),
        None => {
            let cfg = default_config();
            store.set(STORE_KEY, serde_json::to_value(&cfg)?);
            store.save().map_err(|e| AppError::Other(e.to_string()))?;
            Ok(cfg)
        }
    }
}

/// Persist the whole config.
pub fn save<R: Runtime>(app: &AppHandle<R>, cfg: &AppConfig) -> AppResult<()> {
    let store = app.store(STORE_FILE).map_err(|e| AppError::Other(e.to_string()))?;
    store.set(STORE_KEY, serde_json::to_value(cfg)?);
    store.save().map_err(|e| AppError::Other(e.to_string()))?;
    Ok(())
}
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --lib store`
Expected: PASS (4 tests). If the `tauri_plugin_store` `StoreExt`/`get`/`set`/`save` signatures differ for v2.4.3, adjust the adapter only (the pure helpers + their tests are the contract).

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/config/mod.rs src-tauri/src/config/store.rs
git commit -m "feat(config): AppConfig mutation helpers + Tauri store load/save"
```

---

## Task 4: LlmError + retry/auth classification

**Files:**
- Create: `src-tauri/src/llm/error.rs`
- Modify: `src-tauri/src/llm/mod.rs` (add `pub mod error;`)
- Test: inline `#[cfg(test)]` in `error.rs`

- [ ] **Step 1: Wire the module**

Replace the doc-comment-only `llm/mod.rs` body by adding at the top:

```rust
pub mod error;
```

(Keep the existing module doc comment.)

- [ ] **Step 2: Write the failing test**

Create `llm/error.rs` with tests first:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auth_errors_are_not_retryable() {
        for s in [401u16, 403, 404] {
            let e = LlmError::Http { status: s, body: "x".into() };
            assert!(!e.is_retryable(), "{s} should not retry");
            assert!(e.is_auth(), "{s} is auth");
        }
    }

    #[test]
    fn transient_errors_are_retryable() {
        for s in [429u16, 500, 502, 503, 504] {
            assert!(LlmError::Http { status: s, body: "x".into() }.is_retryable());
        }
        assert!(LlmError::Transport("timed out".into()).is_retryable());
    }

    #[test]
    fn parse_errors_are_not_retryable() {
        assert!(!LlmError::Parse("bad json".into()).is_retryable());
    }
}
```

- [ ] **Step 3: Run test to verify it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --lib llm::error`
Expected: FAIL — `LlmError` not defined.

- [ ] **Step 4: Implement LlmError**

Prepend to `error.rs`:

```rust
//! LLM driver error type. Mirrors `llm/anthropic.py:is_retryable_error`.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum LlmError {
    /// Non-2xx HTTP response, carrying the status + a body snippet.
    #[error("HTTP {status}: {body}")]
    Http { status: u16, body: String },
    /// Transport-level failure (timeout, connection reset, DNS).
    #[error("request error: {0}")]
    Transport(String),
    /// Response received but could not be parsed into the expected shape.
    #[error("failed to parse response: {0}")]
    Parse(String),
    /// 2xx response with no usable text content.
    #[error("empty response from LLM")]
    Empty,
}

impl LlmError {
    /// True for transient failures worth retrying (timeouts, 429, 5xx).
    pub fn is_retryable(&self) -> bool {
        match self {
            LlmError::Http { status, .. } => {
                matches!(status, 408 | 425 | 429 | 500 | 502 | 503 | 504)
            }
            LlmError::Transport(_) => true,
            LlmError::Empty => true,
            LlmError::Parse(_) => false,
        }
    }

    /// True for auth/endpoint errors that won't be fixed by retrying.
    pub fn is_auth(&self) -> bool {
        matches!(self, LlmError::Http { status, .. } if matches!(status, 401 | 403 | 404))
    }
}
```

- [ ] **Step 5: Run test to verify it passes**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --lib llm::error`
Expected: PASS.

- [ ] **Step 6: Wire LlmError into AppError (so commands can use `?`)**

In `src-tauri/src/error.rs`, add a variant to `enum AppError` (after `Json`):

```rust
    #[error(transparent)]
    Llm(#[from] crate::llm::error::LlmError),
```

This lets `detect_format(...).await?` (Task 10) convert `LlmError` → `AppError`.
`AppError` still serializes to its `Display` string for the webview (the variant is
`transparent`, delegating to `LlmError`'s message).

- [ ] **Step 7: Build to confirm the new variant compiles**

Run: `cargo build --manifest-path src-tauri/Cargo.toml`
Expected: green.

- [ ] **Step 8: Commit**

```bash
git add src-tauri/src/llm/mod.rs src-tauri/src/llm/error.rs src-tauri/src/error.rs
git commit -m "feat(llm): LlmError with retry/auth classification + AppError conversion"
```

---

## Task 5: LlmDriver trait, shared HTTP helper, and factory

**Files:**
- Modify: `src-tauri/src/llm/mod.rs`
- Test: none yet (covered by driver tasks).

- [ ] **Step 1: Define the trait + helpers**

Add to `llm/mod.rs` (after the module declarations):

```rust
pub mod anthropic;
pub mod detect;
pub mod openai;
pub mod openai_responses;

use async_trait::async_trait;
use reqwest::header::HeaderMap;
use serde_json::Value;

use crate::config::{Connection, Driver};
use error::LlmError;

/// A provider driver: a one-shot completion + a model list. (Streaming et al.
/// arrive with the translation step.)
#[async_trait]
pub trait LlmDriver: Send + Sync {
    async fn complete(&self, system: &str, user: &str) -> Result<String, LlmError>;
    async fn list_models(&self) -> Result<Vec<String>, LlmError>;
    fn model(&self) -> &str;
}

/// Build the driver for a connection. For Custom, callers must resolve the
/// driver via `detect::detect_format` first; this trusts `conn.driver`.
pub fn create_driver(conn: Connection) -> Box<dyn LlmDriver> {
    match conn.driver {
        Driver::Anthropic => Box::new(anthropic::AnthropicDriver::new(conn)),
        Driver::Openai => Box::new(openai::OpenAiDriver::new(conn)),
        Driver::OpenaiResponses => Box::new(openai_responses::OpenAiResponsesDriver::new(conn)),
    }
}

/// POST JSON and return the parsed body, classifying failures into `LlmError`.
pub(crate) async fn post_json(
    client: &reqwest::Client,
    url: &str,
    headers: HeaderMap,
    body: &Value,
    timeout_secs: u64,
) -> Result<Value, LlmError> {
    let resp = client
        .post(url)
        .headers(headers)
        .json(body)
        .timeout(std::time::Duration::from_secs(timeout_secs))
        .send()
        .await
        .map_err(|e| LlmError::Transport(e.to_string()))?;
    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        let snippet: String = body.chars().take(500).collect();
        return Err(LlmError::Http { status: status.as_u16(), body: snippet });
    }
    resp.json::<Value>()
        .await
        .map_err(|e| LlmError::Parse(e.to_string()))
}

/// GET JSON (used by `list_models`).
pub(crate) async fn get_json(
    client: &reqwest::Client,
    url: &str,
    headers: HeaderMap,
    timeout_secs: u64,
) -> Result<Value, LlmError> {
    let resp = client
        .get(url)
        .headers(headers)
        .timeout(std::time::Duration::from_secs(timeout_secs))
        .send()
        .await
        .map_err(|e| LlmError::Transport(e.to_string()))?;
    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        let snippet: String = body.chars().take(500).collect();
        return Err(LlmError::Http { status: status.as_u16(), body: snippet });
    }
    resp.json::<Value>()
        .await
        .map_err(|e| LlmError::Parse(e.to_string()))
}

/// Parse `{ "data": [ { "id": "..." } ] }` model lists (OpenAI + Anthropic shape).
pub(crate) fn parse_model_ids(v: &Value) -> Vec<String> {
    v.get("data")
        .and_then(|d| d.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|m| m.get("id").and_then(|i| i.as_str()).map(String::from))
                .collect()
        })
        .unwrap_or_default()
}

pub(crate) fn base_of(conn: &Connection) -> String {
    conn.base_url.trim_end_matches('/').to_string()
}

pub(crate) fn timeout_of(conn: &Connection) -> u64 {
    conn.timeout.unwrap_or(120) as u64
}
```

- [ ] **Step 2: Verify it compiles (drivers still missing — expect errors only about the driver modules)**

Run: `cargo build --manifest-path src-tauri/Cargo.toml` — will fail until Tasks 6–9 add the driver/detect files. That's expected; proceed to Task 6 (do **not** commit a non-compiling tree). If you want a green checkpoint, temporarily comment the `pub mod` lines for unimplemented files, but the next tasks add them quickly.

> Note for executor: Tasks 5–9 form one compile unit. Implement 5→9, then build + commit once at the end of Task 9. The earlier per-task `cargo test` calls target specific modules and will run once their files exist.

---

## Task 6: Anthropic driver

**Files:**
- Create: `src-tauri/src/llm/anthropic.rs`
- Test: inline `#[cfg(test)]` (wiremock)

- [ ] **Step 1: Write the failing tests**

Create `anthropic.rs` with tests first:

```rust
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
}
```

- [ ] **Step 2: Implement the driver**

Prepend to `anthropic.rs`:

```rust
//! Anthropic Messages API driver. Mirrors `llm/anthropic.py:AnthropicDriver`.

use async_trait::async_trait;
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE};
use serde_json::{json, Value};

use crate::config::Connection;
use crate::llm::error::LlmError;
use crate::llm::{base_of, get_json, parse_model_ids, post_json, timeout_of, LlmDriver};

pub struct AnthropicDriver {
    conn: Connection,
    client: reqwest::Client,
}

impl AnthropicDriver {
    pub fn new(conn: Connection) -> Self {
        Self { conn, client: reqwest::Client::new() }
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
}

#[async_trait]
impl LlmDriver for AnthropicDriver {
    async fn complete(&self, system: &str, user: &str) -> Result<String, LlmError> {
        let url = format!("{}/v1/messages", base_of(&self.conn));
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

    async fn list_models(&self) -> Result<Vec<String>, LlmError> {
        let url = format!("{}/v1/models", base_of(&self.conn));
        let data = get_json(&self.client, &url, self.headers(), timeout_of(&self.conn)).await?;
        Ok(parse_model_ids(&data))
    }

    fn model(&self) -> &str {
        &self.conn.model
    }
}
```

- [ ] **Step 3: (Defer build/test to Task 9 — drivers compile together.)**

---

## Task 7: OpenAI (chat completions) driver

**Files:**
- Create: `src-tauri/src/llm/openai.rs`
- Test: inline `#[cfg(test)]` (wiremock)

- [ ] **Step 1: Write the failing tests**

Create `openai.rs` with tests first:

```rust
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
```

- [ ] **Step 2: Implement the driver**

Prepend to `openai.rs`:

```rust
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
```

- [ ] **Step 2: (Defer build/test to Task 9.)**

---

## Task 8: OpenAI Responses driver

**Files:**
- Create: `src-tauri/src/llm/openai_responses.rs`
- Test: inline `#[cfg(test)]` (wiremock)

- [ ] **Step 1: Write the failing tests**

Create `openai_responses.rs` with tests first:

```rust
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
```

- [ ] **Step 2: Implement the driver**

Prepend to `openai_responses.rs`:

```rust
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
```

- [ ] **Step 2: (Defer build/test to Task 9.)**

---

## Task 9: Custom format auto-detection

**Files:**
- Create: `src-tauri/src/llm/detect.rs`
- Test: inline `#[cfg(test)]` (wiremock)

- [ ] **Step 1: Write the failing tests**

Create `detect.rs` with tests first:

```rust
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
```

- [ ] **Step 2: Implement detection**

Prepend to `detect.rs`:

```rust
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
```

- [ ] **Step 3: Build the whole LLM layer + run all llm tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --lib llm`
Expected: PASS (anthropic 3, openai 2, responses 1, detect 3) and the tree compiles.

- [ ] **Step 4: Commit Tasks 5–9 together**

```bash
git add src-tauri/src/llm/
git commit -m "feat(llm): driver trait + anthropic/openai/responses drivers + format detection"
```

---

## Task 10: Command DTOs + connection commands

**Files:**
- Modify: `src-tauri/src/models/mod.rs` (add DTOs)
- Create: `src-tauri/src/commands/connections.rs`
- Modify: `src-tauri/src/commands/mod.rs` (re-export)
- Modify: `src-tauri/src/lib.rs` (register handlers)
- Test: inline `#[cfg(test)]` in `connections.rs` for the pure builder (`build_connections_view`)

- [ ] **Step 1: Add DTOs**

Append to `models/mod.rs`:

```rust
use crate::config::Driver;

/// One row in the Connections list (no secret material).
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/types/generated/")]
pub struct ConnectionSummary {
    pub name: String,
    pub driver: Driver,
    pub has_key: bool,
}

/// O1 — the Connections list view-model.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/types/generated/")]
pub struct ConnectionsView {
    pub connections: Vec<ConnectionSummary>,
    pub active: String,
    pub personalization: Option<String>,
}

/// O5 — result of a Test.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/types/generated/")]
pub struct TestResult {
    pub ok: bool,
    pub model: String,
    pub detected_driver: Option<Driver>,
    pub message: String,
}

/// O21 — first-run check.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/types/generated/")]
pub struct FirstRunStatus {
    pub has_usable_connection: bool,
}
```

- [ ] **Step 2: Write the failing test for the pure builder**

Create `commands/connections.rs` with the test first:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::presets::default_config;

    #[test]
    fn view_lists_summaries_without_keys() {
        let cfg = default_config();
        let view = build_connections_view(&cfg);
        assert_eq!(view.active, "anthropic");
        assert_eq!(view.personalization.as_deref(), Some("openai"));
        let anthropic = view.connections.iter().find(|c| c.name == "anthropic").unwrap();
        assert!(!anthropic.has_key); // seeded empty
        let ollama = view.connections.iter().find(|c| c.name == "ollama").unwrap();
        assert!(ollama.has_key); // placeholder key
    }
}
```

- [ ] **Step 3: Run test to verify it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --lib commands::connections`
Expected: FAIL — `build_connections_view` not defined.

- [ ] **Step 4: Implement commands + the pure builder**

Prepend to `commands/connections.rs`:

```rust
//! Connection management commands (O1–O5, O21) + presets/model listing.

use tauri::AppHandle;

use crate::config::presets::{presets, Preset};
use crate::config::store;
use crate::config::{AppConfig, Connection, Driver};
use crate::error::{AppError, AppResult};
use crate::llm::detect::detect_format;
use crate::llm::create_driver;
use crate::models::{ConnectionSummary, ConnectionsView, FirstRunStatus, TestResult};

/// Pure: AppConfig -> list view-model (no keys leaked).
pub(crate) fn build_connections_view(cfg: &AppConfig) -> ConnectionsView {
    let connections = cfg
        .connections
        .iter()
        .map(|(name, c)| ConnectionSummary {
            name: name.clone(),
            driver: c.driver,
            has_key: !c.api_key.trim().is_empty(),
        })
        .collect();
    ConnectionsView {
        connections,
        active: cfg.active_connection.clone(),
        personalization: cfg.personalization_model.clone(),
    }
}

#[tauri::command]
pub fn list_connections(app: AppHandle) -> AppResult<ConnectionsView> {
    Ok(build_connections_view(&store::load(&app)?))
}

#[tauri::command]
pub fn read_connection(app: AppHandle, name: String) -> AppResult<Connection> {
    let cfg = store::load(&app)?;
    cfg.connections
        .get(&name)
        .cloned()
        .ok_or_else(|| AppError::Other(format!("unknown connection: {name}")))
}

#[tauri::command]
pub fn save_connection(app: AppHandle, name: String, connection: Connection) -> AppResult<()> {
    let mut cfg = store::load(&app)?;
    store::upsert_connection(&mut cfg, &name, connection);
    store::save(&app, &cfg)
}

#[tauri::command]
pub fn delete_connection(app: AppHandle, name: String) -> AppResult<()> {
    let mut cfg = store::load(&app)?;
    store::remove_connection(&mut cfg, &name)?;
    store::save(&app, &cfg)
}

#[tauri::command]
pub fn set_active_connection(app: AppHandle, name: String) -> AppResult<()> {
    let mut cfg = store::load(&app)?;
    store::set_active(&mut cfg, &name)?;
    store::save(&app, &cfg)
}

#[tauri::command]
pub fn set_personalization_connection(app: AppHandle, name: String) -> AppResult<()> {
    let mut cfg = store::load(&app)?;
    store::set_personalization(&mut cfg, &name)?;
    store::save(&app, &cfg)
}

#[tauri::command]
pub fn first_run_status(app: AppHandle) -> AppResult<FirstRunStatus> {
    Ok(FirstRunStatus {
        has_usable_connection: store::has_usable_connection(&store::load(&app)?),
    })
}

#[tauri::command]
pub fn list_presets() -> Vec<Preset> {
    presets()
}

/// Build a minimal Connection for Test/model-list (thinking + web_search off,
/// tiny max_tokens) so the call is cheap and valid.
fn probe_connection(mut c: Connection, driver: Driver) -> Connection {
    c.driver = driver;
    c.thinking_enabled = Some(false);
    c.web_search = Some(false);
    c.max_tokens = Some(16);
    c
}

#[tauri::command]
pub async fn test_connection(connection: Connection) -> AppResult<TestResult> {
    // Resolve the driver: trust an explicit driver, but for a Custom connection
    // (the UI sends driver=openai as a pre-Test placeholder with no preset) we
    // detect. We always detect when base_url is non-standard? -> Keep it simple:
    // the UI sets `connection.driver` from the preset; for Custom it passes a
    // sentinel via `prompt_template == "__detect__"`. See note below.
    let detect = connection.prompt_template.as_deref() == Some("__detect__");
    let (driver, detected) = if detect {
        let d = detect_format(&connection.base_url, &connection.api_key).await?;
        (d, Some(d))
    } else {
        (connection.driver, None)
    };

    let mut probe = probe_connection(connection, driver);
    probe.prompt_template = None; // strip the sentinel
    let model = probe.model.clone();
    let client = create_driver(probe);
    match client.complete("Reply with OK.", "ping").await {
        Ok(_) => Ok(TestResult {
            ok: true,
            model,
            detected_driver: detected,
            message: match detected {
                Some(d) => format!("✓ Detected {} — responded as {model}", driver_label(d)),
                None => format!("✓ Connection works — responded as {model}"),
            },
        }),
        Err(e) => Ok(TestResult {
            ok: false,
            model,
            detected_driver: detected,
            message: e.to_string(),
        }),
    }
}

#[tauri::command]
pub async fn list_models(connection: Connection) -> AppResult<Vec<String>> {
    let driver = if connection.prompt_template.as_deref() == Some("__detect__") {
        detect_format(&connection.base_url, &connection.api_key).await?
    } else {
        connection.driver
    };
    let mut c = connection;
    c.prompt_template = None;
    c.driver = driver;
    Ok(create_driver(c).list_models().await.unwrap_or_default())
}

fn driver_label(d: Driver) -> &'static str {
    match d {
        Driver::Anthropic => "Anthropic-compatible",
        Driver::Openai | Driver::OpenaiResponses => "OpenAI-compatible",
    }
}
```

> **Detection sentinel note for executor:** the frontend signals "Custom / auto-detect"
> by sending `prompt_template = "__detect__"` on `test_connection` / `list_models`
> (the field is otherwise unused in this step). On Save, the UI persists the
> `detected_driver` returned by Test and clears the sentinel. If you prefer an explicit
> typed flag, add a `detect: bool` parameter to both commands instead — keep it
> consistent across `lib.rs`, `ipc.ts`, and the editor.

- [ ] **Step 5: Re-export + register**

In `commands/mod.rs`, add at top: `pub mod connections;` and `pub use connections::*;` (keep `app_info`).

In `lib.rs`, replace the `invoke_handler` line with:

```rust
        .invoke_handler(tauri::generate_handler![
            commands::app_info,
            commands::list_connections,
            commands::read_connection,
            commands::save_connection,
            commands::delete_connection,
            commands::set_active_connection,
            commands::set_personalization_connection,
            commands::first_run_status,
            commands::list_presets,
            commands::test_connection,
            commands::list_models,
        ])
```

- [ ] **Step 6: Run tests + build**

Run: `cargo test --manifest-path src-tauri/Cargo.toml`
Expected: PASS (all suites). Then `cargo build --manifest-path src-tauri/Cargo.toml` — green.

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/models/mod.rs src-tauri/src/commands/ src-tauri/src/lib.rs
git commit -m "feat(commands): connection CRUD, test_connection, list_models, first_run_status"
```

---

## Task 11: Generate + verify ts-rs bindings

**Files:**
- Modify: `package.json` (scripts)
- Generated: `src/types/generated/*.ts`

- [ ] **Step 1: Add scripts**

In `package.json` `"scripts"`, add:

```json
    "gen:routes": "tsr generate",
    "gen:bindings": "cargo test --manifest-path src-tauri/Cargo.toml export_bindings"
```

> ts-rs writes bindings as a side effect of the `export_bindings_*` tests it generates;
> running the full test suite also writes them. If `export_bindings` matches nothing,
> use `cargo test --manifest-path src-tauri/Cargo.toml` instead.

- [ ] **Step 2: Generate**

Run: `bun run gen:bindings` (or `cargo test --manifest-path src-tauri/Cargo.toml`).
Expected: `src/types/generated/` now contains `Preset.ts`, `ConnectionsView.ts`,
`ConnectionSummary.ts`, `TestResult.ts`, `FirstRunStatus.ts`, and updated
`Connection.ts` (with `prompt_template`, `thinking_glossary_norm_budget`).

- [ ] **Step 3: Verify**

Run: `ls src/types/generated` and confirm the new files exist; open `Connection.ts` and
confirm the two new optional fields are present.

- [ ] **Step 4: Commit**

```bash
git add package.json src/types/generated/
git commit -m "chore: gen:bindings/gen:routes scripts + regenerate ts-rs bindings"
```

---

## Task 12: Theme tokens + Phosphor + scripts

**Files:**
- Modify: `src/index.css`
- Modify: `package.json` (dependency)

> **Verification gate (frontend):** `bun run build`. No unit tests.

- [ ] **Step 1: Install Phosphor**

Run: `bun add @phosphor-icons/react`

- [ ] **Step 2: Apply the `theme.py` palette**

In `src/index.css`, replace the values inside the `.dark { … }` block (and `:root` for
parity) so the shadcn variables resolve to the design-system tokens. Use these mappings
(keep the existing `@theme inline` and `@layer base` blocks):

```css
.dark {
    --background: #14161a;        /* bg_base */
    --foreground: #d8d8d8;        /* text_primary */
    --card: #1a1c20;              /* bg_surface */
    --card-foreground: #d8d8d8;
    --popover: #1f2226;          /* bg_raised */
    --popover-foreground: #d8d8d8;
    --primary: #4a9eff;          /* accent */
    --primary-foreground: #0f1114;
    --secondary: #1f2226;        /* bg_raised */
    --secondary-foreground: #d8d8d8;
    --muted: #1a1c20;
    --muted-foreground: #888888; /* text_muted */
    --accent: #1f2226;           /* bg_raised (hover surfaces) */
    --accent-foreground: #ffffff;
    --destructive: #ff8a8a;      /* danger */
    --border: #2d3036;
    --input: #3a3d44;            /* border_strong */
    --ring: #4a9eff;
    --sidebar: #0f1114;          /* bg_deepest (rail) */
    --sidebar-foreground: #d8d8d8;
    --sidebar-primary: #4a9eff;
    --sidebar-primary-foreground: #0f1114;
    --sidebar-accent: #1f2226;
    --sidebar-accent-foreground: #ffffff;
    --sidebar-border: #2d3036;
    --sidebar-ring: #4a9eff;
}
```

Also add semantic tokens used by chips/state, after the `.dark` block:

```css
@theme inline {
    --color-alert: #ffcc55;
    --color-success: #8ad08a;
    --color-danger: #ff8a8a;
    --color-state-cleanup: #b48ead;
    --color-state-verify: #79c0c0;
    --color-bg-deepest: #0f1114;
    --color-bg-hover: #2a2d33;
}
```

Ensure the app forces dark: the `ThemeProvider` already defaults to `dark` (see
`main.tsx`); no change needed.

- [ ] **Step 3: Verify build**

Run: `bun run build`
Expected: succeeds (route-gen → tsc → vite). The app renders in dark with the new palette.

- [ ] **Step 4: Commit**

```bash
git add package.json bun.lock src/index.css
git commit -m "feat(ui): theme.py token palette + @phosphor-icons/react"
```

---

## Task 13: Design-system primitive components

**Files:**
- Create: `src/components/help-text.tsx`, `setup-field.tsx`, `status-chip.tsx`, `state-chip.tsx`, `empty-state.tsx`, `section-help.tsx`

> Verification: `bun run build`. Build them with the tokens above; small + presentational.

- [ ] **Step 1: HelpText + SetupField**

`help-text.tsx`:

```tsx
import { Info } from "@phosphor-icons/react";

/** One-line muted helper under an input. */
export function HelpText({ children }: { children: React.ReactNode }) {
  return (
    <p className="mt-1 flex items-start gap-1 text-[11px] leading-snug text-muted-foreground">
      <Info weight="fill" className="mt-px size-3 shrink-0 text-primary" />
      <span>{children}</span>
    </p>
  );
}
```

`setup-field.tsx`:

```tsx
import type { ReactNode } from "react";

/** Labelled control + optional helper text. */
export function SetupField({
  label,
  htmlFor,
  help,
  children,
}: {
  label: string;
  htmlFor?: string;
  help?: ReactNode;
  children: ReactNode;
}) {
  return (
    <div className="mb-3.5">
      <label htmlFor={htmlFor} className="mb-1 block text-[11.5px] font-semibold">
        {label}
      </label>
      {children}
      {help}
    </div>
  );
}
```

- [ ] **Step 2: StatusChip + StateChip**

`status-chip.tsx`:

```tsx
import { cn } from "@/lib/utils";

type Variant = "muted" | "alert" | "danger" | "success" | "accent";

const VARIANT: Record<Variant, string> = {
  muted: "text-muted-foreground border-border",
  alert: "text-[color:var(--color-alert)] border-[color:var(--color-alert)]/40",
  danger: "text-[color:var(--color-danger)] border-[color:var(--color-danger)]/40",
  success: "text-[color:var(--color-success)] border-[color:var(--color-success)]/40",
  accent: "text-primary border-primary/40",
};

export function StatusChip({
  variant = "muted",
  children,
  className,
  ...rest
}: { variant?: Variant; children: React.ReactNode } & React.HTMLAttributes<HTMLSpanElement>) {
  return (
    <span
      className={cn(
        "inline-flex items-center gap-1 rounded-[13px] border px-2.5 py-0.5 text-[11px] tabular-nums",
        VARIANT[variant],
        className,
      )}
      {...rest}
    >
      {children}
    </span>
  );
}
```

`state-chip.tsx` — maps `TranslationState` → color (only `idle` used now; full map for later):

```tsx
const STATE_COLOR: Record<string, string> = {
  idle: "var(--muted-foreground)",
  pending: "var(--muted-foreground)",
  translating: "var(--primary)",
  retranslating: "var(--color-alert)",
  cleanup: "var(--color-state-cleanup)",
  verifying: "var(--color-state-verify)",
  done: "var(--color-success)",
  warning: "var(--color-alert)",
  failed: "var(--color-danger)",
};

export function StateChip({ state = "idle", label }: { state?: string; label?: string }) {
  const color = STATE_COLOR[state] ?? STATE_COLOR.idle;
  return (
    <span className="inline-flex items-center gap-1.5 rounded-[13px] border border-border px-2.5 py-0.5 text-[11px]">
      <span className="size-2 rounded-full" style={{ background: color }} />
      {label ?? state[0].toUpperCase() + state.slice(1)}
    </span>
  );
}
```

- [ ] **Step 3: EmptyState + SectionHelp**

`empty-state.tsx`:

```tsx
import type { ReactNode } from "react";

export function EmptyState({
  icon,
  title,
  description,
  action,
}: { icon?: ReactNode; title: string; description?: string; action?: ReactNode }) {
  return (
    <div className="flex h-full flex-col items-center justify-center gap-2 p-10 text-center">
      {icon}
      <h2 className="text-base font-semibold text-foreground">{title}</h2>
      {description ? (
        <p className="max-w-sm text-sm text-muted-foreground">{description}</p>
      ) : null}
      {action}
    </div>
  );
}
```

`section-help.tsx`:

```tsx
import { useState } from "react";
import { CaretRight } from "@phosphor-icons/react";

/** A collapsible section (used by "Advanced settings"). */
export function SectionHelp({
  title,
  hint,
  children,
  defaultOpen = false,
}: { title: string; hint?: string; children: React.ReactNode; defaultOpen?: boolean }) {
  const [open, setOpen] = useState(defaultOpen);
  return (
    <div className="mt-2 border-t border-border pt-2">
      <button
        type="button"
        onClick={() => setOpen((o) => !o)}
        className="flex w-full items-center gap-1.5 text-[11.5px] text-muted-foreground hover:text-foreground"
      >
        <CaretRight className={cnRotate(open)} />
        <span className="font-medium">{title}</span>
        {hint ? <span className="text-muted-foreground/70">{hint}</span> : null}
      </button>
      {open ? <div className="mt-2">{children}</div> : null}
    </div>
  );
}

function cnRotate(open: boolean) {
  return open ? "size-3.5 rotate-90 transition-transform" : "size-3.5 transition-transform";
}
```

- [ ] **Step 4: Verify build**

Run: `bun run build`
Expected: succeeds (these are unused-but-valid until wired). If tree-shaking complains
about unused exports, ignore — they're imported in later tasks.

- [ ] **Step 5: Commit**

```bash
git add src/components/help-text.tsx src/components/setup-field.tsx src/components/status-chip.tsx src/components/state-chip.tsx src/components/empty-state.tsx src/components/section-help.tsx
git commit -m "feat(ui): design-system primitives (chips, fields, empty state, section help)"
```

---

## Task 14: Shell — NavRail, layout, header, status bar + routing

**Files:**
- Create: `src/components/nav-rail.tsx`
- Modify: `src/components/app-layout.tsx`, `src/components/status-bar.tsx`, `src/components/page-header.tsx`
- Modify: `src/routes/index.tsx`, `src/routes/settings.tsx`
- Create: `src/routes/connections.tsx` (placeholder for now), `src/routes/help.tsx`, `src/routes/project.tsx`
- Modify/keep: `src/routes/{glossary,translate,verify}.tsx` (gated placeholders)
- Modify: `src/stores/app-store.ts`

> Verification: `bun run build` + `bun tauri dev` smoke (rail renders; first-run redirects to Connections).

- [ ] **Step 1: Extend the app store**

Replace `stores/app-store.ts` body's interface/impl to add connection + first-run UI state:

```ts
import { create } from "zustand";

interface AppState {
  workdir: string | null;
  sourceLang: string;
  targetLang: string;
  activeConnection: string | null;
  hasUsableConnection: boolean;
  setWorkdir: (dir: string | null) => void;
  setLanguages: (source: string, target: string) => void;
  setActiveConnection: (name: string | null) => void;
  setHasUsableConnection: (v: boolean) => void;
}

export const useAppStore = create<AppState>((set) => ({
  workdir: null,
  sourceLang: "zh",
  targetLang: "en",
  activeConnection: null,
  hasUsableConnection: false,
  setWorkdir: (workdir) => set({ workdir }),
  setLanguages: (sourceLang, targetLang) => set({ sourceLang, targetLang }),
  setActiveConnection: (activeConnection) => set({ activeConnection }),
  setHasUsableConnection: (hasUsableConnection) => set({ hasUsableConnection }),
}));
```

- [ ] **Step 2: NavRail**

Create `components/nav-rail.tsx`:

```tsx
import {
  BookOpen, CheckCircle, Folder, Gear, Lightning, Play, Question,
  type Icon,
} from "@phosphor-icons/react";
import { Link, useRouterState } from "@tanstack/react-router";
import { useAppStore } from "@/stores/app-store";
import { cn } from "@/lib/utils";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";

interface RailItem {
  to: string;
  label: string;
  icon: Icon;
  group: "workflow" | "setup";
  needsFolder?: boolean;
}

const ITEMS: RailItem[] = [
  { to: "/project", label: "Project", icon: Folder, group: "workflow", needsFolder: true },
  { to: "/glossary", label: "Glossary", icon: BookOpen, group: "workflow", needsFolder: true },
  { to: "/translate", label: "Translate", icon: Play, group: "workflow", needsFolder: true },
  { to: "/verify", label: "Verify", icon: CheckCircle, group: "workflow", needsFolder: true },
  { to: "/connections", label: "Connections", icon: Lightning, group: "setup" },
  { to: "/settings", label: "Settings", icon: Gear, group: "setup" },
  { to: "/help", label: "Help", icon: Question, group: "setup" },
];

export function NavRail() {
  const workdir = useAppStore((s) => s.workdir);
  const hasUsableConnection = useAppStore((s) => s.hasUsableConnection);
  const pathname = useRouterState({ select: (s) => s.location.pathname });

  const workflow = ITEMS.filter((i) => i.group === "workflow");
  const setup = ITEMS.filter((i) => i.group === "setup");

  const render = (item: RailItem) => {
    const disabled = item.needsFolder && !workdir;
    const active = pathname.startsWith(item.to);
    const Icon = item.icon;
    const body = (
      <div
        className={cn(
          "flex w-16 flex-col items-center gap-1 rounded-md py-2 text-[10px]",
          active && "bg-[color:var(--popover)] text-primary",
          disabled ? "cursor-not-allowed text-muted-foreground/50" : "hover:bg-[color:var(--color-bg-hover)]",
        )}
      >
        <Icon weight={active ? "fill" : "regular"} className="size-5" />
        {item.label}
        {item.to === "/connections" ? (
          <span className={hasUsableConnection ? "text-[color:var(--color-success)]" : "text-[color:var(--color-alert)]"}>
            {hasUsableConnection ? "✓" : "⚠"}
          </span>
        ) : null}
      </div>
    );
    if (disabled) {
      return (
        <Tooltip key={item.to}>
          <TooltipTrigger asChild><div>{body}</div></TooltipTrigger>
          <TooltipContent side="right">Open a folder first</TooltipContent>
        </Tooltip>
      );
    }
    return <Link key={item.to} to={item.to}>{body}</Link>;
  };

  return (
    <nav className="flex w-20 flex-col items-center gap-1 border-r border-border bg-[color:var(--sidebar)] py-3">
      {workflow.map(render)}
      <div className="my-1 h-px w-10 bg-border" />
      {render(setup[0])}
      <div className="flex-1" />
      {setup.slice(1).map(render)}
    </nav>
  );
}
```

- [ ] **Step 3: AppLayout (shell grid)**

Replace `components/app-layout.tsx`:

```tsx
import type { ReactNode } from "react";
import { NavRail } from "@/components/nav-rail";
import { StatusBar } from "@/components/status-bar";

export function AppLayout({ children }: { children: ReactNode }) {
  return (
    <div className="grid h-screen grid-cols-[auto_1fr] grid-rows-[1fr_auto] bg-background text-foreground">
      <div className="row-span-2"><NavRail /></div>
      <main className="min-h-0 overflow-auto">{children}</main>
      <StatusBar />
    </div>
  );
}
```

- [ ] **Step 4: StatusBar (reworked)**

Replace `components/status-bar.tsx`:

```tsx
import { useQuery } from "@tanstack/react-query";
import { Folder } from "@phosphor-icons/react";
import { StateChip } from "@/components/state-chip";
import { StatusChip } from "@/components/status-chip";
import { Separator } from "@/components/ui/separator";
import { ipc } from "@/lib/ipc";
import { useAppStore } from "@/stores/app-store";

export function StatusBar() {
  const workdir = useAppStore((s) => s.workdir);
  const sourceLang = useAppStore((s) => s.sourceLang);
  const targetLang = useAppStore((s) => s.targetLang);
  const connection = useAppStore((s) => s.activeConnection);
  const { data: appInfo } = useQuery({ queryKey: ["app-info"], queryFn: ipc.appInfo });

  return (
    <footer className="col-start-2 flex h-8 items-center gap-3 border-t border-border bg-[color:var(--color-bg-deepest)] px-3 text-[11px] text-muted-foreground">
      <span className="flex min-w-0 items-center gap-1.5">
        <Folder className="size-3.5 shrink-0" />
        <span className="truncate">{workdir ?? "No folder selected"}</span>
      </span>
      <Separator orientation="vertical" className="h-4" />
      <span className="shrink-0 tabular-nums">— files · — lines</span>
      <span className="flex-1" />
      <span className="shrink-0">{sourceLang}→{targetLang}</span>
      <StatusChip variant="accent">{connection ?? "No connection"}</StatusChip>
      <StateChip state="idle" />
      <span className="shrink-0 opacity-60">core {appInfo?.version ?? "…"}</span>
    </footer>
  );
}
```

- [ ] **Step 5: PageHeader (reworked, minimal for step 1)**

Replace `components/page-header.tsx`:

```tsx
interface PageHeaderProps {
  title: string;
  description?: string;
  actions?: React.ReactNode;
}

export function PageHeader({ title, description, actions }: PageHeaderProps) {
  return (
    <div className="flex items-start justify-between gap-4 border-b border-border px-5 py-3">
      <div className="flex flex-col gap-0.5">
        <h1 className="text-base font-semibold tracking-tight text-foreground">{title}</h1>
        {description ? (
          <p className="text-[12.5px] text-muted-foreground">{description}</p>
        ) : null}
      </div>
      {actions}
    </div>
  );
}
```

- [ ] **Step 6: Routes — first-run redirect + placeholders**

Replace `routes/index.tsx`:

```tsx
import { createFileRoute, redirect } from "@tanstack/react-router";
import { ipc } from "@/lib/ipc";
import { EmptyState } from "@/components/empty-state";

export const Route = createFileRoute("/")({
  beforeLoad: async () => {
    const status = await ipc.firstRunStatus();
    if (!status.has_usable_connection) {
      throw redirect({ to: "/connections" });
    }
  },
  component: () => (
    <EmptyState
      title="Open a folder of subtitles to begin"
      description="Folder pickup arrives in the next step. For now, manage your AI providers in Connections."
    />
  ),
});
```

Create `routes/help.tsx` and `routes/project.tsx`, and ensure `glossary/translate/verify`
exist as gated placeholders. Each placeholder follows this shape (substitute title):

```tsx
import { createFileRoute } from "@tanstack/react-router";
import { EmptyState } from "@/components/empty-state";

export const Route = createFileRoute("/help")({
  component: () => <EmptyState title="Help" description="Coming soon." />,
});
```

Replace `routes/settings.tsx` similarly with an `EmptyState title="Settings"`.
For `project/glossary/translate/verify`, the rail already blocks them without a folder;
the route component can render `<EmptyState title="…" description="Open a folder first." />`.

- [ ] **Step 7: Add `firstRunStatus` to ipc (minimal, expanded in Task 15)**

In `lib/ipc.ts`, add to the `ipc` object:

```ts
  firstRunStatus: () => invoke<import("@/types/generated/FirstRunStatus").FirstRunStatus>("first_run_status"),
```

- [ ] **Step 8: Verify build + smoke**

Run: `bun run build`
Expected: succeeds. Then `bun tauri dev` (manual): the rail renders; with an empty store
the app redirects to `/connections`; workflow rail items are dimmed with a tooltip.

- [ ] **Step 9: Commit**

```bash
git add src/components/ src/routes/ src/stores/app-store.ts src/lib/ipc.ts
git commit -m "feat(ui): shell — nav rail, layout grid, status bar, header, gated routes"
```

---

## Task 15: IPC wrappers + connection hooks

**Files:**
- Modify: `src/lib/ipc.ts`
- Create: `src/features/connections/use-connections.ts`

> Verification: `bun run build`.

- [ ] **Step 1: Complete the IPC surface**

Replace `lib/ipc.ts`'s `ipc` object with the full set (keep `onBackendEvent`):

```ts
import { invoke } from "@tauri-apps/api/core";
import { listen, type EventCallback, type UnlistenFn } from "@tauri-apps/api/event";
import type { AppInfo } from "@/types/generated/AppInfo";
import type { Connection } from "@/types/generated/Connection";
import type { ConnectionsView } from "@/types/generated/ConnectionsView";
import type { Preset } from "@/types/generated/Preset";
import type { TestResult } from "@/types/generated/TestResult";
import type { FirstRunStatus } from "@/types/generated/FirstRunStatus";

export const ipc = {
  appInfo: () => invoke<AppInfo>("app_info"),
  firstRunStatus: () => invoke<FirstRunStatus>("first_run_status"),
  listPresets: () => invoke<Preset[]>("list_presets"),
  listConnections: () => invoke<ConnectionsView>("list_connections"),
  readConnection: (name: string) => invoke<Connection>("read_connection", { name }),
  saveConnection: (name: string, connection: Connection) =>
    invoke<void>("save_connection", { name, connection }),
  deleteConnection: (name: string) => invoke<void>("delete_connection", { name }),
  setActiveConnection: (name: string) => invoke<void>("set_active_connection", { name }),
  setPersonalizationConnection: (name: string) =>
    invoke<void>("set_personalization_connection", { name }),
  testConnection: (connection: Connection) =>
    invoke<TestResult>("test_connection", { connection }),
  listModels: (connection: Connection) => invoke<string[]>("list_models", { connection }),
};

export function onBackendEvent<T>(name: string, handler: EventCallback<T>): Promise<UnlistenFn> {
  return listen<T>(name, handler);
}
```

- [ ] **Step 2: Connection hooks (TanStack Query)**

Create `features/connections/use-connections.ts`:

```ts
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import type { Connection } from "@/types/generated/Connection";
import { ipc } from "@/lib/ipc";
import { useAppStore } from "@/stores/app-store";

const KEY = ["connections"] as const;

export function useConnections() {
  const setActive = useAppStore((s) => s.setActiveConnection);
  const setHasUsable = useAppStore((s) => s.setHasUsableConnection);
  return useQuery({
    queryKey: KEY,
    queryFn: async () => {
      const view = await ipc.listConnections();
      setActive(view.active);
      setHasUsable(view.connections.some((c) => c.has_key));
      return view;
    },
  });
}

export function usePresets() {
  return useQuery({ queryKey: ["presets"], queryFn: ipc.listPresets, staleTime: Infinity });
}

export function useConnection(name: string | null) {
  return useQuery({
    queryKey: ["connection", name],
    queryFn: () => ipc.readConnection(name as string),
    enabled: !!name,
  });
}

export function useConnectionMutations() {
  const qc = useQueryClient();
  const invalidate = () => qc.invalidateQueries({ queryKey: KEY });
  return {
    save: useMutation({
      mutationFn: ({ name, connection }: { name: string; connection: Connection }) =>
        ipc.saveConnection(name, connection),
      onSuccess: invalidate,
    }),
    remove: useMutation({ mutationFn: ipc.deleteConnection, onSuccess: invalidate }),
    setActive: useMutation({ mutationFn: ipc.setActiveConnection, onSuccess: invalidate }),
    setPersonalization: useMutation({
      mutationFn: ipc.setPersonalizationConnection, onSuccess: invalidate,
    }),
    test: useMutation({ mutationFn: ipc.testConnection }),
    listModels: useMutation({ mutationFn: ipc.listModels }),
  };
}
```

- [ ] **Step 3: Verify build**

Run: `bun run build`
Expected: succeeds.

- [ ] **Step 4: Commit**

```bash
git add src/lib/ipc.ts src/features/connections/use-connections.ts
git commit -m "feat(connections): typed IPC wrappers + TanStack Query hooks"
```

---

## Task 16: Connections view (list + editor + model combobox)

**Files:**
- Create: `src/features/connections/connections-page.tsx`, `connection-list.tsx`, `connection-editor.tsx`, `model-combobox.tsx`
- Modify: `src/routes/connections.tsx` (mount the page)

> Verification: `bun run build` + manual smoke (select/add/edit/test/save).

- [ ] **Step 1: Model combobox**

Create `features/connections/model-combobox.tsx`:

```tsx
import { useState } from "react";
import { CaretDown } from "@phosphor-icons/react";
import { Command, CommandEmpty, CommandGroup, CommandInput, CommandItem, CommandList } from "@/components/ui/command";
import { Popover, PopoverContent, PopoverTrigger } from "@/components/ui/popover";
import { Button } from "@/components/ui/button";

/** Free-typing combobox over curated ∪ live-fetched model ids. */
export function ModelCombobox({
  value, onChange, options,
}: { value: string; onChange: (v: string) => void; options: string[] }) {
  const [open, setOpen] = useState(false);
  const merged = Array.from(new Set([value, ...options].filter(Boolean)));
  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger asChild>
        <Button variant="outline" role="combobox" className="w-full justify-between font-normal">
          {value || "Select or type a model…"}
          <CaretDown className="size-4 opacity-60" />
        </Button>
      </PopoverTrigger>
      <PopoverContent className="w-[var(--radix-popover-trigger-width)] p-0">
        <Command>
          <CommandInput
            placeholder="Search or type a model…"
            value={value}
            onValueChange={onChange}
          />
          <CommandList>
            <CommandEmpty>Use “{value}” (custom)</CommandEmpty>
            <CommandGroup>
              {merged.map((m) => (
                <CommandItem key={m} value={m} onSelect={(v) => { onChange(v); setOpen(false); }}>
                  {m}
                </CommandItem>
              ))}
            </CommandGroup>
          </CommandList>
        </Command>
      </PopoverContent>
    </Popover>
  );
}
```

> If `components/ui/popover.tsx` is absent, add it: `bunx shadcn@latest add popover`
> (the project uses the maia style; accept defaults). `command` already exists.

- [ ] **Step 2: Connection list**

Create `features/connections/connection-list.tsx`:

```tsx
import { Plus } from "@phosphor-icons/react";
import type { ConnectionsView } from "@/types/generated/ConnectionsView";
import { cn } from "@/lib/utils";

export function ConnectionList({
  view, selected, onSelect, onAdd,
}: {
  view: ConnectionsView | undefined;
  selected: string | null;
  onSelect: (name: string) => void;
  onAdd: () => void;
}) {
  return (
    <div className="w-52 shrink-0 border-r border-border bg-[color:var(--color-bg-deepest)] p-2.5">
      {view?.connections.map((c) => (
        <button
          key={c.name}
          type="button"
          onClick={() => onSelect(c.name)}
          className={cn(
            "mb-0.5 flex w-full items-center gap-2 rounded-md px-2.5 py-1.5 text-left text-xs",
            selected === c.name ? "bg-[color:var(--popover)] text-foreground" : "text-muted-foreground hover:bg-[color:var(--color-bg-hover)]",
          )}
        >
          <span className="truncate capitalize">{c.name}</span>
          {view.active === c.name ? (
            <span className="ml-auto text-[9px] text-[color:var(--color-success)]">● active</span>
          ) : null}
        </button>
      ))}
      <button
        type="button"
        onClick={onAdd}
        className="mt-1 flex w-full items-center gap-1.5 border-t border-border px-2.5 py-2 text-[11px] text-muted-foreground hover:text-foreground"
      >
        <Plus className="size-3.5" /> Add connection
      </button>
    </div>
  );
}
```

- [ ] **Step 3: Connection editor**

Create `features/connections/connection-editor.tsx`. It drives a `react-hook-form` over
`Connection`, with Provider preset select, API key + Test, model combobox, personalization
checkbox, Advanced section, and footer. Detection sentinel: when the selected preset is
`custom`, set `prompt_template = "__detect__"` on the payload sent to `testConnection` /
`listModels`; after a successful Test, persist `detected_driver` into the form's `driver`
and clear the sentinel before Save.

```tsx
import { useEffect, useMemo, useState } from "react";
import { useForm } from "react-hook-form";
import { Eye, EyeSlash } from "@phosphor-icons/react";
import type { Connection } from "@/types/generated/Connection";
import type { Preset } from "@/types/generated/Preset";
import type { TestResult } from "@/types/generated/TestResult";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Checkbox } from "@/components/ui/checkbox";
import { SetupField } from "@/components/setup-field";
import { HelpText } from "@/components/help-text";
import { SectionHelp } from "@/components/section-help";
import { ModelCombobox } from "./model-combobox";

const EMPTY: Connection = {
  driver: "openai", base_url: "", api_key: "", model: "",
  max_tokens: 16000, batch_dialogue_limit: 100, timeout: 120, connect_timeout: 10,
  concurrency: 5, thinking_enabled: null, thinking_budget: null, web_search: null,
  prompt_template: null, thinking_glossary_norm_budget: null,
} as unknown as Connection;

export function ConnectionEditor({
  name, initial, presets, isActive, isPersonalization,
  onSave, onSetActive, onSetPersonalization, onRemove, onTest, onListModels,
}: {
  name: string;
  initial: Connection | undefined;
  presets: Preset[];
  isActive: boolean;
  isPersonalization: boolean;
  onSave: (name: string, c: Connection) => Promise<void> | void;
  onSetActive: (name: string) => void;
  onSetPersonalization: (name: string) => void;
  onRemove: (name: string) => void;
  onTest: (c: Connection) => Promise<TestResult>;
  onListModels: (c: Connection) => Promise<string[]>;
}) {
  const { register, handleSubmit, watch, setValue, reset } = useForm<Connection>({
    defaultValues: initial ?? EMPTY,
  });
  useEffect(() => reset(initial ?? EMPTY), [initial, name, reset]);

  const [presetKey, setPresetKey] = useState<string>("");
  const [revealKey, setRevealKey] = useState(false);
  const [testState, setTestState] = useState<"idle" | "testing" | TestResult>("idle");
  const [liveModels, setLiveModels] = useState<string[]>([]);

  const current = watch();
  const isCustom = presetKey === "custom";
  const curated = useMemo(
    () => presets.find((p) => p.key === presetKey)?.models ?? [],
    [presets, presetKey],
  );

  const applyPreset = (key: string) => {
    setPresetKey(key);
    const p = presets.find((x) => x.key === key);
    if (!p) return;
    if (p.driver) setValue("driver", p.driver);
    setValue("base_url", p.base_url);
    if (p.model) setValue("model", p.model);
  };

  const withDetectSentinel = (c: Connection): Connection =>
    isCustom ? { ...c, prompt_template: "__detect__" } : c;

  const runTest = async () => {
    setTestState("testing");
    const res = await onTest(withDetectSentinel(current));
    if (res.detected_driver) setValue("driver", res.detected_driver);
    setTestState(res);
  };

  const refreshModels = async () => {
    try { setLiveModels(await onListModels(withDetectSentinel(current))); } catch { /* keep curated */ }
  };

  return (
    <form
      className="flex flex-1 flex-col"
      onSubmit={handleSubmit((c) => onSave(name, { ...c, prompt_template: null }))}
    >
      <div className="flex-1 space-y-1 overflow-auto p-4">
        <SetupField label="Provider" help={<HelpText>Pick your provider — we fill in the technical bits for you.</HelpText>}>
          <select
            className="h-9 w-full rounded-md border border-input bg-[color:var(--card)] px-2 text-sm"
            value={presetKey}
            onChange={(e) => applyPreset(e.target.value)}
          >
            <option value="">— choose —</option>
            {presets.map((p) => <option key={p.key} value={p.key}>{p.label}</option>)}
          </select>
        </SetupField>

        <SetupField label="API key" help={<HelpText>Stored locally on your computer only — never uploaded.</HelpText>}>
          <div className="flex gap-2">
            <div className="relative flex-1">
              <Input type={revealKey ? "text" : "password"} placeholder="••••••••" {...register("api_key")} />
              <button type="button" onClick={() => setRevealKey((v) => !v)} className="absolute right-2 top-1/2 -translate-y-1/2 text-muted-foreground">
                {revealKey ? <EyeSlash className="size-4" /> : <Eye className="size-4" />}
              </button>
            </div>
            <Button type="button" variant="secondary" onClick={runTest}>Test</Button>
          </div>
          {testState === "testing" ? <p className="mt-1 text-[11px] text-muted-foreground">Testing…</p> : null}
          {typeof testState === "object" ? (
            <p className={"mt-1 text-[11px] " + (testState.ok ? "text-[color:var(--color-success)]" : "text-[color:var(--color-danger)]")}>
              {testState.message}
            </p>
          ) : null}
        </SetupField>

        <SetupField label="Model">
          <ModelCombobox
            value={current.model ?? ""}
            onChange={(v) => setValue("model", v)}
            options={Array.from(new Set([...curated, ...liveModels]))}
          />
          <button type="button" onClick={refreshModels} className="mt-1 text-[11px] text-primary hover:underline">
            Refresh model list
          </button>
        </SetupField>

        <label className="mb-2 flex items-center gap-2 text-[11.5px]">
          <Checkbox
            checked={isPersonalization}
            onCheckedChange={() => onSetPersonalization(name)}
          />
          Use this connection for “look up names online”
        </label>
        <HelpText>The web-lookup step needs a model that can search the web (e.g. OpenAI/Gemini).</HelpText>

        <SectionHelp title="Advanced settings" hint="(address, tokens, parallelism, timeouts, thinking, web search)">
          <div className="grid grid-cols-2 gap-2 text-[11px]">
            <Field label="Base URL" {...register("base_url")} />
            <Field label="Max tokens" type="number" {...register("max_tokens", { valueAsNumber: true })} />
            <Field label="Batch dialogue limit" type="number" {...register("batch_dialogue_limit", { valueAsNumber: true })} />
            <Field label="Concurrency" type="number" {...register("concurrency", { valueAsNumber: true })} />
            <Field label="Timeout (s)" type="number" {...register("timeout", { valueAsNumber: true })} />
            <Field label="Connect timeout (s)" type="number" {...register("connect_timeout", { valueAsNumber: true })} />
          </div>
          {isCustom ? <HelpText>API format auto-detected on Test (currently: {current.driver}).</HelpText> : null}
        </SectionHelp>
      </div>

      <div className="flex items-center gap-2 border-t border-border bg-[color:var(--popover)] px-4 py-3">
        <Button type="button" variant="ghost" className="text-[color:var(--color-danger)]" onClick={() => onRemove(name)}>Remove</Button>
        <div className="flex-1" />
        {!isActive ? <Button type="button" variant="secondary" onClick={() => onSetActive(name)}>Set as active</Button> : null}
        <Button type="submit">Save</Button>
      </div>
    </form>
  );
}

function Field({ label, ...rest }: { label: string } & React.InputHTMLAttributes<HTMLInputElement>) {
  return (
    <label className="flex flex-col gap-1">
      <span className="text-muted-foreground">{label}</span>
      <Input className="h-8" {...rest} />
    </label>
  );
}
```

- [ ] **Step 4: Connections page (wires list + editor + first-run banner)**

Create `features/connections/connections-page.tsx`:

```tsx
import { useEffect, useState } from "react";
import { PageHeader } from "@/components/page-header";
import {
  useConnections, useConnection, usePresets, useConnectionMutations,
} from "./use-connections";
import { ConnectionList } from "./connection-list";
import { ConnectionEditor } from "./connection-editor";

export function ConnectionsPage() {
  const { data: view } = useConnections();
  const { data: presets } = usePresets();
  const m = useConnectionMutations();
  const [selected, setSelected] = useState<string | null>(null);
  const { data: initial } = useConnection(selected);

  useEffect(() => {
    if (!selected && view?.connections.length) setSelected(view.active || view.connections[0].name);
  }, [view, selected]);

  const firstRun = view && !view.connections.some((c) => c.has_key);

  return (
    <div className="flex h-full flex-col">
      <PageHeader title="LLM Connections" description="An AI provider does the translating. Pick one, paste a key, test." />
      {firstRun ? (
        <div className="border-b border-border bg-[color:var(--popover)] px-5 py-2 text-[12.5px] text-primary">
          Welcome — let’s connect an AI provider so you can start translating.
        </div>
      ) : null}
      <div className="flex min-h-0 flex-1">
        <ConnectionList
          view={view}
          selected={selected}
          onSelect={setSelected}
          onAdd={() => setSelected(`new-${Date.now()}`)}
        />
        {selected && presets ? (
          <ConnectionEditor
            name={selected}
            initial={initial}
            presets={presets}
            isActive={view?.active === selected}
            isPersonalization={view?.personalization === selected}
            onSave={async (name, c) => { await m.save.mutateAsync({ name, connection: c }); setSelected(name); }}
            onSetActive={(name) => m.setActive.mutate(name)}
            onSetPersonalization={(name) => m.setPersonalization.mutate(name)}
            onRemove={(name) => { m.remove.mutate(name); setSelected(null); }}
            onTest={(c) => m.test.mutateAsync(c)}
            onListModels={(c) => m.listModels.mutateAsync(c)}
          />
        ) : null}
      </div>
    </div>
  );
}
```

- [ ] **Step 5: Mount the route**

Replace `routes/connections.tsx`:

```tsx
import { createFileRoute } from "@tanstack/react-router";
import { ConnectionsPage } from "@/features/connections/connections-page";

export const Route = createFileRoute("/connections")({ component: ConnectionsPage });
```

- [ ] **Step 6: Verify build + smoke**

Run: `bun run build`
Expected: succeeds. Then `bun tauri dev` (manual): select a seeded connection, paste a
key, Test (against a real provider), Save, Set active; the rail Connections badge flips
to ✓ and the status-bar chip updates. Adding "Custom" with a base URL + Test detects the
format.

- [ ] **Step 7: Commit**

```bash
git add src/features/connections/ src/routes/connections.tsx
git commit -m "feat(connections): list + editor + model combobox + Test wiring"
```

---

## Task 17: Final verification

**Files:** none (verification only).

- [ ] **Step 1: Backend tests + build**

Run: `cargo test --manifest-path src-tauri/Cargo.toml` → all pass.
Run: `cargo build --manifest-path src-tauri/Cargo.toml` → green.

- [ ] **Step 2: Frontend build**

Run: `bun run build` → green (route-gen → tsc → vite).

- [ ] **Step 3: Manual smoke (bun tauri dev)**

Confirm: empty store → lands on Connections (first-run banner, rail ⚠); save a real key →
Test ✓, rail flips to ✓, status-bar chip updates; workflow rail items dimmed with "Open a
folder first"; Settings/Help show placeholders; restart preserves connections (store
persistence).

- [ ] **Step 4: Commit any final fixes**

```bash
git add -A
git commit -m "chore: step 1 — shell + connections complete"
```

---

## Self-review notes (for the executor)

- **Detection sentinel** (`prompt_template == "__detect__"`) is the one cross-cutting
  contract: it appears in Task 10 (commands), Task 15 (ipc passes Connection through),
  and Task 16 (editor sets/clears it). Keep all three in sync; if you switch to an
  explicit `detect: bool` arg, change all three.
- **`max_tokens` for Test** is forced to 16 server-side (`probe_connection`), so a saved
  connection's large `max_tokens` doesn't make Test slow/expensive.
- **Frontend tasks are build-verified, not unit-tested** (no runner configured) — this is
  intentional per the spec's testing strategy.
- If `popover`/`command` maia components are missing, add them via shadcn CLI before
  Task 16 step 1.
