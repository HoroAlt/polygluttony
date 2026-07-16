//! Config persistence. Pure helpers over `AppConfig` (unit-tested) and a
//! thin JSON-file adapter that seeds defaults on first run.

use std::path::{Path, PathBuf};

use crate::config::{presets::default_config, AppConfig, Connection};
use crate::error::{AppError, AppResult};

pub const CONFIG_FILE: &str = "config.json";

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

pub fn clear_personalization(cfg: &mut AppConfig) {
    cfg.personalization_model = None;
}

pub fn remove_connection(cfg: &mut AppConfig, name: &str) -> AppResult<()> {
    if cfg.active_connection == name {
        return Err(AppError::Other(
            "reassign the active connection before removing it".into(),
        ));
    }
    cfg.connections.remove(name);
    if cfg.personalization_model.as_deref() == Some(name) {
        cfg.personalization_model = None;
    }
    Ok(())
}

pub fn rename_connection(cfg: &mut AppConfig, old: &str, new: &str) -> AppResult<()> {
    let new = new.trim();
    if new.is_empty() {
        return Err(AppError::Other("connection name cannot be empty".into()));
    }
    if old == new {
        return Ok(());
    }
    if !cfg.connections.contains_key(old) {
        return Err(AppError::Other(format!("unknown connection: {old}")));
    }
    if cfg.connections.contains_key(new) {
        return Err(AppError::Other(format!(
            "a connection named '{new}' already exists"
        )));
    }
    if let Some(conn) = cfg.connections.remove(old) {
        cfg.connections.insert(new.to_string(), conn);
    }
    if cfg.active_connection == old {
        cfg.active_connection = new.to_string();
    }
    if cfg.personalization_model.as_deref() == Some(old) {
        cfg.personalization_model = Some(new.to_string());
    }
    Ok(())
}

/// A connection is usable when it has a non-empty API key OR its base URL
/// points to localhost (e.g. an Ollama instance that needs no key).
///
/// For the CLI we tighten the localhost check: only `http://localhost`,
/// `http://127.0.0.1`, or `http://[::1]`, optionally with a port. This
/// blocks accidental exfiltration when the user pastes a random URL.
pub fn connection_is_usable(conn: &Connection) -> bool {
    if !conn.api_key.trim().is_empty() {
        return true;
    }
    is_localhost_url(&conn.base_url)
}

fn is_localhost_url(url: &str) -> bool {
    let lower = url.trim().to_lowercase();
    let prefix = if let Some(rest) = lower.strip_prefix("http://") {
        rest
    } else if let Some(rest) = lower.strip_prefix("https://") {
        // HTTPS to a "localhost"-ish name is still local-only for the
        // local-API case; the user opted in to the warning.
        rest
    } else {
        return false;
    };
    let host_end = prefix
        .find(|c: char| c == '/' || c == ':' || c == '?' || c == '#')
        .unwrap_or(prefix.len());
    let host = &prefix[..host_end];
    host == "localhost" || host == "127.0.0.1" || host == "[::1]" || host == "::1"
}

pub fn has_usable_connection(cfg: &AppConfig) -> bool {
    cfg.connections.values().any(connection_is_usable)
}

pub fn set_default_languages(cfg: &mut AppConfig, source: &str, target: &str) {
    cfg.default_source = source.to_string();
    cfg.default_target = target.to_string();
}

/// Load the config from `<data_dir>/config.json`, seeding + persisting
/// defaults on first run.
pub fn load(data_dir: &Path) -> AppResult<AppConfig> {
    let path = config_path(data_dir);
    if path.exists() {
        let raw = std::fs::read_to_string(&path).map_err(|e| AppError::Io(e))?;
        let cfg: AppConfig = serde_json::from_str(&raw).map_err(AppError::from)?;
        Ok(cfg)
    } else {
        let cfg = default_config();
        save(data_dir, &cfg)?;
        Ok(cfg)
    }
}

pub fn save(data_dir: &Path, cfg: &AppConfig) -> AppResult<()> {
    std::fs::create_dir_all(data_dir).map_err(|e| AppError::Io(e))?;
    let path = config_path(data_dir);
    let s = serde_json::to_string_pretty(cfg).map_err(AppError::from)?;
    // Atomic-ish write: write to a sibling temp file, then rename.
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, s).map_err(|e| AppError::Io(e))?;
    std::fs::rename(&tmp, &path).map_err(|e| AppError::Io(e))?;
    Ok(())
}

fn config_path(data_dir: &Path) -> PathBuf {
    data_dir.join(CONFIG_FILE)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Driver;

    fn sample() -> Connection {
        Connection {
            driver: Driver::Openai,
            base_url: "u".into(),
            api_key: "k".into(),
            model: "m".into(),
            max_tokens: None,
            batch_dialogue_limit: None,
            timeout: None,
            connect_timeout: None,
            concurrency: None,
            thinking_enabled: None,
            thinking_budget: None,
            thinking_glossary_budget: None,
            thinking_glossary_norm_budget: None,
            web_search: None,
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
        assert!(remove_connection(&mut cfg, "anthropic").is_err());
        assert!(remove_connection(&mut cfg, "google").is_ok());
        assert!(!cfg.connections.contains_key("google"));
    }

    #[test]
    fn rename_moves_entry_and_updates_references() {
        let mut cfg = default_config();
        rename_connection(&mut cfg, "anthropic", "claude").unwrap();
        assert!(cfg.connections.contains_key("claude"));
        assert_eq!(cfg.active_connection, "claude");
        rename_connection(&mut cfg, "openai", "gpt").unwrap();
        assert_eq!(cfg.personalization_model.as_deref(), Some("gpt"));
        assert!(rename_connection(&mut cfg, "google", "claude").is_err());
        assert!(rename_connection(&mut cfg, "google", "  ").is_err());
        assert!(rename_connection(&mut cfg, "google", "google").is_ok());
    }

    #[test]
    fn removing_personalization_connection_clears_the_reference() {
        let mut cfg = default_config();
        assert_eq!(cfg.personalization_model.as_deref(), Some("openai"));
        remove_connection(&mut cfg, "openai").unwrap();
        assert_eq!(cfg.personalization_model, None);
    }

    #[test]
    fn first_run_detection() {
        let cfg = default_config();
        assert!(has_usable_connection(&cfg));
        let mut empty = default_config();
        for c in empty.connections.values_mut() {
            c.api_key.clear();
        }
        assert!(has_usable_connection(&empty));
    }

    #[test]
    fn connection_is_usable_rules() {
        let mut c = Connection {
            driver: Driver::Openai,
            base_url: "https://api.example.com".into(),
            api_key: String::new(),
            model: "m".into(),
            max_tokens: None,
            batch_dialogue_limit: None,
            timeout: None,
            connect_timeout: None,
            concurrency: None,
            thinking_enabled: None,
            thinking_budget: None,
            thinking_glossary_budget: None,
            thinking_glossary_norm_budget: None,
            web_search: None,
        };
        assert!(!connection_is_usable(&c));
        c.api_key = "sk-test".into();
        assert!(connection_is_usable(&c));
        c.api_key.clear();
        c.base_url = "http://localhost:11434".into();
        assert!(connection_is_usable(&c));
        c.base_url = "http://127.0.0.1:11434".into();
        assert!(connection_is_usable(&c));
    }

    #[test]
    fn load_or_seed_round_trips_through_disk() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = load(dir.path()).unwrap();
        assert!(has_usable_connection(&cfg));
        save(dir.path(), &cfg).unwrap();
        let again = load(dir.path()).unwrap();
        assert_eq!(cfg.active_connection, again.active_connection);
    }
}
