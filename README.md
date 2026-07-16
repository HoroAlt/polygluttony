# anitranslate

Local-first CLI for translating `.ass`/`.srt` subtitle files via LLM.

## Why

Anime/donghua subtitles in the wrong language are everywhere. This CLI
extracts them from `.ass`/`.srt` files (or `.mkv` containers) and translates
through whatever OpenAI-compatible endpoint you point it at — local Ollama
by default, but also Anthropic, OpenAI, Gemini, DeepSeek, OpenRouter.

## Features

- **Local-first**: Ollama on `localhost:11434` works without an API key.
- **ASS byte-faithful**: `{\pos}`, `{\an8}`, fonts, styles are preserved.
- **Line-marker / partial-failure recovery**: every input line gets a
  marker; the LLM sometimes drops or merges lines, and the engine
  salvages the correct prefix.
- **Drift detection**: a five-signal weighted detector catches translations
  that wander off mid-batch, and the affected scope is retranslated.
- **Cross-episode glossary**: 6 categories, auto-detected world type
  (xianxia / wuxia / historical / modern), 6-pass LLM glossary build.
- **No Tauri, no Electron, no Node**: pure Rust workspace. One binary, no
  binary blobs you can't audit.
- **255 unit tests passing** on the translation engine.

## Quick start

```bash
# Build
git clone https://github.com/HoroAlt/anitranslate.git
cd anitranslate
cargo build --release

# Or via Docker (Ollama bundled):
docker compose up -d ollama
docker compose run --rm anitranslate translate -f /work -s en -t ru
```

The binary lands at `target/release/anitranslate`. Add it to your `PATH`.

## Subcommands

```bash
anitranslate translate     # translate every .ass/.srt in a folder
anitranslate build-glossary
anitranslate inspect
anitranslate config [--path|--show-active]
```

Run `anitranslate --help` for full options.

## Configuration

The config file lives at `$ANITRANSLATE_DATA_DIR/config.json` (default
`~/.local/share/anitranslate/` on Linux,
`~/Library/Application Support/anitranslate/` on macOS).

First run seeds three connections: `ollama` (localhost:11434), `anthropic`,
and `openai`. Edit the config to set API keys, switch `active_connection`,
or add custom endpoints.

## Security

- `unsafe_code = "forbid"` workspace lint.
- No `eval`, no `child_process`, no `Command::new`, no Tauri runtime.
- No telemetry. The LLM is the only network endpoint.
- Co-authored-by trailer on AI-assisted commits.

## License

MIT. See `LICENSE`.

## Acknowledgements

Forked from
[blyat-uk/polygluttony](https://github.com/blyat-uk/polygluttony) (MIT).
Engine ported; Tauri shell, React UI, and TypeScript bindings removed.
