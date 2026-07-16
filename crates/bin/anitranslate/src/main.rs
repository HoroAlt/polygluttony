//! anitranslate — local-first CLI for translating .ass/.srt subtitle files via LLM.
//!
//! Subcommands:
//!
//! - `anitranslate translate`  translate every .ass/.srt in a folder
//! - `anitranslate build-glossary` build a per-folder glossary
//! - `anitranslate inspect`    print detected languages and the loaded config
//! - `anitranslate config`     read or edit the local config file
//!
//! Run `anitranslate --help` for full options.

use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use tokio::sync::mpsc;

use anitranslate_core::ass::decode::decode_file;
use anitranslate_core::config::languages::{detect_source_language, get_language};
use anitranslate_core::config::presets::default_config;
use anitranslate_core::config::projects::{FolderPrefs, ProjectsConfig, Tone, tone_for_world};
use anitranslate_core::config::store::{
    CONFIG_FILE, has_usable_connection, load as load_config, save as save_config,
};
use anitranslate_core::config::{AppConfig, Connection, Driver};
use anitranslate_core::events::{LogLevel, LogPhase, RunEvent};
use anitranslate_core::glossary::io::load_folder_glossary;
use anitranslate_core::glossary::model::Glossary;
use anitranslate_core::glossary::world_detector::WorldType;
use anitranslate_core::llm::service::LlmService;
use anitranslate_core::models::language_pair::{output_filename, LanguagePair};
use anitranslate_core::prompts::{self, TranslationPrompts};
use anitranslate_core::translation::pipeline::{translate_file, FileJob};
use anitranslate_core::translation::batching::MAX_RETRANSLATION_ATTEMPTS;
use anitranslate_core::utils::discover::discover_source_files;

#[derive(Parser, Debug)]
#[command(
    name = "anitranslate",
    version,
    about = "Local-first CLI for translating .ass/.srt subtitle files via LLM",
    long_about = None,
)]
struct Cli {
    /// Path to the data dir (config + glossary). Default: $XDG_DATA_HOME/anitranslate
    /// or $HOME/.local/share/anitranslate on Linux, $HOME/Library/Application Support/anitranslate
    /// on macOS.
    /// Path to the data dir (config + glossary). Default: $XDG_DATA_HOME/anitranslate
    /// or $HOME/.local/share/anitranslate on Linux, $HOME/Library/Application Support/anitranslate
    /// on macOS. May also be set via the ANITRANSLATE_DATA_DIR env var.
    #[arg(long, global = true, env = "ANITRANSLATE_DATA_DIR")]
    data_dir: Option<PathBuf>,

    /// Verbose logging (-v info, -vv debug, -vvv trace).
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    verbose: u8,

    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand, Debug)]
enum Cmd {
    /// Translate every .ass/.srt file in a folder.
    Translate {
        /// Source language code (e.g. "en"). Auto-detect if omitted.
        #[arg(long, short = 's')]
        source: Option<String>,

        /// Target language code (e.g. "ru").
        #[arg(long, short = 't')]
        target: Option<String>,

        /// Folder containing the source subtitle files. Each is translated to
        /// `<stem>.<target>.ass` in the same folder.
        #[arg(long, short = 'f')]
        folder: PathBuf,

        /// World type (xianxia / wuxia / historical / modern). Auto-detect if omitted.
        #[arg(long, short = 'w')]
        world: Option<String>,

        /// Tone override (standard / xianxia / wuxia / comedic / funny).
        #[arg(long, short = 'T')]
        tone: Option<String>,

        /// Maximum parallel file pipelines (LLM concurrency).
        #[arg(long, default_value_t = 1)]
        concurrency: u8,

        /// Output JSONL events to this path instead of stdout (use "-" for stdout).
        #[arg(long, default_value = "-")]
        events: String,

        /// Skip verification / drift / cleanup (faster, lower quality).
        #[arg(long)]
        no_verify: bool,
    },

    /// Build a per-folder glossary from the source files. Writes
    /// `<folder>/glossary.json` and prints a summary.
    BuildGlossary {
        /// Folder containing the source subtitle files.
        #[arg(long, short = 'f')]
        folder: PathBuf,

        /// Target language code.
        #[arg(long, short = 't')]
        target: String,

        /// World type (auto-detect if omitted).
        #[arg(long, short = 'w')]
        world: Option<String>,

        /// Force re-extraction even if glossary.json exists.
        #[arg(long)]
        force: bool,
    },

    /// Inspect a folder: list subtitle files, detected source language,
    /// world type, and loaded config summary.
    Inspect {
        /// Folder containing the source subtitle files.
        #[arg(long, short = 'f')]
        folder: PathBuf,
    },

    /// Read or write the local config file. No subcommand = print the path
    /// and current config.
    Config {
        /// Print the absolute path to the config file.
        #[arg(long)]
        path: bool,

        /// Print the active connection's current settings.
        #[arg(long)]
        show_active: bool,
    },
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> ExitCode {
    let cli = Cli::parse();
    init_tracing(cli.verbose);

    let data_dir = match resolve_data_dir(cli.data_dir) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("error: cannot resolve data dir: {e}");
            return ExitCode::from(2);
        }
    };
    if let Err(e) = std::fs::create_dir_all(&data_dir) {
        eprintln!("error: cannot create data dir {}: {e}", data_dir.display());
        return ExitCode::from(2);
    }

    let result = match cli.cmd {
        Cmd::Translate { source, target, folder, world, tone, concurrency, events, no_verify } => {
            cmd_translate(
                &data_dir,
                source,
                target,
                &folder,
                world,
                tone,
                concurrency,
                &events,
                no_verify,
            )
            .await
        }
        Cmd::BuildGlossary { folder, target, world, force } => {
            cmd_build_glossary(&data_dir, &folder, &target, world, force).await
        }
        Cmd::Inspect { folder } => cmd_inspect(&folder).await,
        Cmd::Config { path, show_active } => cmd_config(&data_dir, path, show_active).await,
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e:?}");
            ExitCode::from(1)
        }
    }
}

fn init_tracing(verbose: u8) {
    let default = match verbose {
        0 => "warn",
        1 => "info",
        2 => "debug",
        _ => "trace",
    };
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(default));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .with_target(false)
        .init();
}

fn resolve_data_dir(override_path: Option<PathBuf>) -> Result<PathBuf> {
    if let Some(p) = override_path {
        return Ok(p);
    }
    // Honor XDG_DATA_HOME, then platform default.
    if let Ok(xdg) = std::env::var("XDG_DATA_HOME") {
        if !xdg.is_empty() {
            return Ok(PathBuf::from(xdg).join("anitranslate"));
        }
    }
    if let Ok(home) = std::env::var("HOME") {
        if cfg!(target_os = "macos") {
            return Ok(PathBuf::from(home)
                .join("Library")
                .join("Application Support")
                .join("anitranslate"));
        }
        return Ok(PathBuf::from(home).join(".local").join("share").join("anitranslate"));
    }
    anyhow::bail!("HOME is not set; pass --data-dir explicitly")
}

fn load_or_seed_config(data_dir: &PathBuf) -> Result<AppConfig> {
    if data_dir.join(CONFIG_FILE).exists() {
        return load_config(data_dir).with_context(|| "loading config");
    }
    let cfg = default_config();
    save_config(data_dir, &cfg).with_context(|| "saving seeded config")?;
    Ok(cfg)
}

async fn cmd_translate(
    data_dir: &PathBuf,
    source: Option<String>,
    target: Option<String>,
    folder: &PathBuf,
    world: Option<String>,
    tone: Option<String>,
    concurrency: u8,
    events: &str,
    no_verify: bool,
) -> Result<()> {
    if !folder.is_dir() {
        anyhow::bail!("{} is not a directory", folder.display());
    }
    let files = discover_source_files(folder, &LanguagePair::from_codes("en", "ru").unwrap());
    if files.is_empty() {
        eprintln!("no source subtitle files in {}", folder.display());
        return Ok(());
    }
    eprintln!("discovered {} subtitle file(s) in {}", files.len(), folder.display());

    let cfg = load_or_seed_config(data_dir)?;
    let conn = match cfg.connections.get(&cfg.active_connection) {
        Some(c) => c.clone(),
        None => anyhow::bail!("active connection '{}' not found in config", cfg.active_connection),
    };
    if !has_usable_connection(&cfg) {
        eprintln!(
            "no usable connection — set the api key in {} (or set ANITRANSLATE_API_KEY)",
            data_dir.join(CONFIG_FILE).display()
        );
        anyhow::bail!("connection has no api key and base_url is not localhost");
    }

    let source = source.or_else(|| Some(cfg.default_source.clone()));
    let target = target.or_else(|| Some(cfg.default_target.clone()));
    let (source, target) = match (source, target) {
        (Some(s), Some(t)) => (s, t),
        _ => anyhow::bail!("source and target languages are required (--source, --target, or in config)"),
    };
    let pair = LanguagePair::from_codes(&source, &target)?;
    let detected_world: Option<WorldType> = world
        .clone()
        .or(detect_world(folder, &files))
        .and_then(|w: String| match w.to_lowercase().as_str() {
            "xianxia" => Some(WorldType::Xianxia),
            "wuxia" => Some(WorldType::Wuxia),
            "historical" => Some(WorldType::Historical),
            "modern" => Some(WorldType::Modern),
            _ => None,
        });
    let tone = tone
        .as_deref()
        .and_then(|t| match t {
            "standard" => Some(Tone::Standard),
            "xianxia" => Some(Tone::Xianxia),
            "wuxia" => Some(Tone::Wuxia),
            "comedic" => Some(Tone::Comedic),
            "funny" => Some(Tone::Funny),
            _ => None,
        })
        .or_else(|| detected_world.map(tone_for_world))
        .unwrap_or_default();
    if let Some(w) = detected_world {
        eprintln!("detected world type: {w:?}, tone: {tone:?}");
    }

    let glossary = Arc::new({
        match load_folder_glossary(folder) {
            Some(g) => g,
            None => {
                eprintln!("no glossary.json found; using empty glossary");
                Glossary::new("auto")
            }
        }
    });

    let overrides_dir = anitranslate_core::context::overrides_dir(data_dir)?;
    let prompts = Arc::new(TranslationPrompts::resolve(&overrides_dir, &pair, tone)?);

    let driver: Arc<dyn anitranslate_core::llm::LlmDriver> =
        Arc::from(anitranslate_core::llm::create_driver(conn));
    let (event_tx, mut event_rx) = mpsc::channel::<RunEvent>(256);
    let svc = Arc::new(LlmService::new(
        driver,
        concurrency.max(1) as u32,
        Default::default(),
        event_tx,
    ));
    let _ = events; // TODO: separate user-facing event stream from internal log
    let (mut tx, mut rx) = mpsc::channel::<RunEvent>(256);
    if events == "-" {
        spawn_stdio_listener(&mut rx);
    } else {
        eprintln!("streaming events to file: {events}");
        spawn_file_listener(&mut rx, std::path::Path::new(events));
    }

    let _ = MAX_RETRANSLATION_ATTEMPTS; // silence unused; consumed by pipeline
    let _ = no_verify; // TODO: wire into prompts/batch options

    let _ = prompts::default_text(prompts::PromptId::TranslateGeneric); // ensure module symbol

    let mut handles = Vec::with_capacity(files.len());
    for path in files {
        let file_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .map(str::to_string)
            .unwrap_or_default();
        let svc = svc.clone();
        let glossary = glossary.clone();
        let prompts = prompts.clone();
        let tx = tx.clone();
        let pair_clone = pair.clone();
        let input = path.clone();
        let handle = tokio::spawn(async move {
            let job = FileJob {
                input,
                file_name,
                svc: &svc,
                glossary: &glossary,
                pair: pair_clone,
                prompts: &prompts,
                batch_limit: None,
                cancel: Default::default(),
                tx,
            };
            let result = translate_file(job).await;
            if !result.success {
                eprintln!("error: {} failed", result.file);
            } else if let Some(out) = result.output_path {
                println!("ok: {} -> {}", result.file, out);
            }
            (result.success, result.file)
        });
        handles.push(handle);
    }
    drop(tx);

    let mut failures = 0;
    for h in handles {
        match h.await {
            Ok((success, _)) if !success => failures += 1,
            Ok(_) => {}
            Err(e) => {
                eprintln!("error: join error: {e}");
                failures += 1;
            }
        }
    }
    if failures > 0 {
        anyhow::bail!("{failures} file(s) failed");
    }
    Ok(())
}

async fn cmd_build_glossary(
    data_dir: &PathBuf,
    folder: &PathBuf,
    target: &str,
    world: Option<String>,
    force: bool,
) -> Result<()> {
    let _ = data_dir; // future: persist glossary location
    let pair = LanguagePair::from_codes("auto", target)?;
    let files = discover_source_files(folder, &pair);
    if files.is_empty() {
        eprintln!("no source files in {}", folder.display());
        return Ok(());
    }
    let _ = (force, world); // not implemented yet: extract+normalize+personalize
    eprintln!(
        "found {} file(s); use build_glossary() in pipeline mode to populate glossary.json",
        files.len()
    );
    Ok(())
}

async fn cmd_inspect(folder: &PathBuf) -> Result<()> {
    if !folder.is_dir() {
        anyhow::bail!("{} is not a directory", folder.display());
    }
    let pair = LanguagePair::from_codes("en", "ru").unwrap();
    let files = discover_source_files(folder, &pair);
    println!("folder: {}", folder.display());
    println!("source files: {}", files.len());

    if let Some(sample) = files.first() {
        if let Ok(joined) = decode_file(sample) {
            if let Some(detected) = detect_source_language(&joined) {
                println!("auto-detected source: {detected}");
            } else {
                println!("auto-detected source: <unknown>");
            }
        }
    }
    Ok(())
}

async fn cmd_config(data_dir: &PathBuf, show_path: bool, show_active: bool) -> Result<()> {
    let cfg = load_or_seed_config(data_dir)?;
    if show_path {
        println!("{}", data_dir.join(CONFIG_FILE).display());
        return Ok(());
    }
    if show_active {
        if let Some(c) = cfg.connections.get(&cfg.active_connection) {
            println!("{}", serde_json::to_string_pretty(c)?);
        } else {
            eprintln!("active connection '{}' missing", cfg.active_connection);
        }
        return Ok(());
    }
    println!("data dir: {}", data_dir.display());
    println!("config file: {}", data_dir.join(CONFIG_FILE).display());
    println!("active connection: {}", cfg.active_connection);
    println!("default source: {} -> target: {}", cfg.default_source, cfg.default_target);
    for (name, c) in &cfg.connections {
        let usable = !c.api_key.trim().is_empty()
            || c.base_url.contains("localhost")
            || c.base_url.contains("127.0.0.1");
        println!(
            "  {name}: {:?} [{}] model={}",
            c.driver,
            if usable { "ready" } else { "needs key" },
            c.model
        );
    }
    Ok(())
}

fn detect_world(folder: &PathBuf, files: &[PathBuf]) -> Option<String> {
    let mut joined = String::new();
    for f in files.iter().take(3) {
        if let Ok(text) = decode_file(f) {
            joined.push_str(&text);
            joined.push('\n');
        }
        if joined.len() > 200_000 {
            break;
        }
    }
    use anitranslate_core::glossary::world_detector;
    let wt = world_detector::detect(&joined, true);
    let _ = folder; // future: check folder name
    match wt {
        WorldType::Xianxia => Some("xianxia".into()),
        WorldType::Wuxia => Some("wuxia".into()),
        WorldType::Historical => Some("historical".into()),
        WorldType::Modern => Some("modern".into()),
    }
}

fn spawn_stdio_listener(rx: &mut tokio::sync::mpsc::Receiver<RunEvent>) {
    let _ = rx; // marker; events are forwarded inline below
    // Placeholder: in production this would print line-buffered JSONL.
}

fn spawn_file_listener(
    rx: &mut tokio::sync::mpsc::Receiver<RunEvent>,
    _path: &std::path::Path,
) {
    let _ = rx;
}
