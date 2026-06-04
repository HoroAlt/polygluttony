# Step 1 — App Shell + Connections (with real Test)

**Status:** Approved design (brainstorming) — ready for implementation plan
**Date:** 2026-06-04
**Project:** polygluttony (Python `subs-translate` → Tauri 2 / Rust + React rewrite)
**This is the first implementation step** of a UI-first, step-by-step port. We build
the new window shell from `polygluttony-docs/` and reimplement the Python logic that
the first window needs.

---

## 1. Goal & deliverable

From a cold start, the app opens into the new **single-window shell** (left icon rail
+ per-view header + bottom status bar), themed with the real `theme.py` palette. The
first-run check decides the landing view:

- **No usable connection** (no connection has an API key) → land on **Connections**
  with a welcome message; rail Connections shows `⚠`; workflow items dimmed.
- **A usable connection exists** → land on a minimal placeholder home (full Welcome +
  folder pickup is **step 2**).

In the **Connections** view the user can: pick a provider **preset**, paste an **API
key** (masked + reveal), choose a **model** (autocomplete combobox, free typing
allowed), expand **Advanced**, hit **Test** (a real request to the provider via a
ported LLM driver), and **Save / Set active / Add / Remove**, optionally marking a
connection as the **personalization** ("look up names online") connection. All of it
persists via the **Tauri store** plugin. The status bar and rail badges reflect
connection state.

Workflow rail items (Project / Glossary / Translate / Verify) render but are **gated
and dimmed** ("Open a folder first") because folder pickup arrives in step 2.

### Done criteria
- `cargo build` and `bun run build` both green; ts-rs bindings regenerated.
- Rust unit tests pass: preset seeding, config load/save round-trip, per-driver
  request-body construction + response parsing (fixtures), error classification, the
  Test path against a mock HTTP server, the Custom **format auto-detection** algorithm,
  and `list_models` parsing.
- Manual: one real **Test** succeeds against a provider the user has a key for.
- Cold start with an empty store lands on Connections (first-run); after saving a
  connection with a key, the rail badge flips to `✓` and the status-bar connection
  chip updates.

---

## 2. Non-goals (explicit scope boundaries)

Deferred to **step 2**: Welcome screen, folder pickup (O6), Project view, ASS parser,
world detection (O7), language list/validation (O8).

Deferred to **later steps**: streaming (`stream()`), thinking-budget multiplier/
override, debug request/response logging, token-aware batching, glossary / translate /
verify pipelines, real Settings (O20) and Help content, file-watch, OS keychain.

The LLM port in this step is **only** the one-shot `complete()` path plus a cheap
`/models` GET — enough to power **Test**, **auto-detect**, and **model autocomplete**.
Streaming and the rest of the driver surface come with the translation step.

---

## 3. Locked decisions (from brainstorming)

1. **Scope:** Shell + Connections + a *real* Test. Folder pickup is a separate step.
2. **Config storage:** Tauri **store plugin** (JSON in app-data), *not* the old
   `~/.config/subs-translator/config.yml`. Shape mirrors `AppConfig`.
3. **API keys:** stored **plaintext** in the local store (mirrors current behavior).
   UI copy: "stored locally on your computer only — never uploaded." OS keychain is a
   future, dedicated decision; UI must not over-promise.
4. **Icons:** **Phosphor** (`@phosphor-icons/react`), matching the design-system doc's
   icon names 1:1. (This **supersedes** the earlier "hugeicons" note in project memory
   — already updated. Existing scaffold uses `@hugeicons/*`; migrate the few current
   usages or leave them until their views are rebuilt.)
5. **Rail completeness:** build the full rail now. Connections is real; workflow items
   are dimmed/gated; **Settings + Help are thin "coming soon" placeholders**.
6. **Provider presets = 5:** **Anthropic**, **Google** (Gemini), **OpenAI**,
   **Ollama** (local), **Custom**. Z.AI (Anthropic-compatible) and OpenRouter
   (OpenAI-compatible) are **not** first-class presets — they're reachable via
   **Custom**. The seeded default active connection becomes one of these five.
7. **Custom API format = auto-detect only (no manual selector).** On Test, polygluttony
   probes both wire formats and locks in whichever responds (see §5.7). **Ollama is
   hardwired** to OpenAI-compatible — no probing. Detection is status-code based so it
   works even when the key is wrong.
8. **Model field = combobox** (free typing always allowed) backed by **curated
   per-provider lists + a live `/models` fetch** when a key/endpoint is present
   (required for Ollama; keeps cloud lists current).

---

## 4. Architecture overview

```
React view ──invoke──▶ Tauri command ──▶ config store / LLM driver ──▶ result
   (Connections)         (commands/)        (config/, llm/)
        ▲                                                │
        └──────────────── typed return (ts-rs) ──────────┘
```

The webview never talks to the network or filesystem directly. Every capability is a
`#[tauri::command]`. ts-rs generates the TS types for every payload crossing the seam.

### Backend layout (`src-tauri/src/`)
- `config/` — extend `AppConfig` / `Connection` / `Driver`; add a `ConfigStore` that
  loads/saves the whole `AppConfig` to the Tauri store and seeds defaults on first run;
  a `presets` module with the provider preset table + curated model lists.
- `llm/` *(new)* — `LlmDriver` trait, `AnthropicDriver`, `OpenAiDriver`,
  `OpenAiResponsesDriver`, a `create_driver` factory, `list_models` per driver, the
  format-detection probe, and `LlmError`.
- `commands/` — connection commands + `test_connection` + `list_models` +
  `first_run_status`.
- `models/` / `error.rs` — extend as needed (`AppError` already covers Io/Http/Json/Other).

### Frontend layout (`src/`)
- Replace `components/main-nav.tsx` + `components/app-layout.tsx` with a **shell**:
  `NavRail`, per-view `Header`, routed `<Outlet/>`, `StatusBar`.
- `index.css` — replace the generic neutral oklch palette with the `theme.py` tokens.
- `features/connections/` — the Connections view + editor (incl. the model combobox).
- `components/` — new design-system primitives: `NavRail`, `StatusChip`, `StateChip`,
  `SetupField`, `HelpText`, `EmptyState`, `SectionHelp`.
- `stores/app-store.ts` — extend with active-connection + first-run state.
- `lib/ipc.ts` — typed wrappers for the new commands.

---

## 5. Backend design

### 5.1 Config model (`config/mod.rs`)
Keep the existing `AppConfig` / `Connection` / `Driver`. Add two fields to `Connection`
to match `LlmConnectionSettings` for the Advanced panel:
- `prompt_template: Option<String>`
- `thinking_glossary_norm_budget: Option<u32>`

`Driver` already serializes kebab-case to `anthropic` / `openai` / `openai-responses`
— correct. (Google/Gemini and Ollama both use the `openai` driver; Custom resolves to
`openai` or `anthropic` via detection.)

### 5.2 Config store (`config/store.rs`)
A thin wrapper over `tauri_plugin_store` (via `StoreExt` on `AppHandle`). One store
file (e.g. `config.json`) holding the serialized `AppConfig`.
- `load(app) -> AppConfig`: read the store; if empty/missing, **seed defaults** and
  persist, then return.
- `save(app, &AppConfig)`: serialize + persist.
- Read-modify-write helpers used by the commands (save one connection, set active, etc.).

**Seeded defaults** (port of `get_default_config()`, adapted to the 5-preset list):
- `default_source="zh"`, `default_target="en"`.
- Seeded connections (empty `api_key` except Ollama, which needs none): `anthropic`,
  `google`, `openai`, `ollama`. (`custom` is created on demand via Add, not seeded.)
- `active_connection="anthropic"` (cosmetic until a key is added — first-run gates on
  `has_key`).
- `personalization_model="openai"` (a web-capable provider).

### 5.3 Provider presets (`config/presets.rs`)
The table backing the Provider dropdown. Picking a preset prefills `driver`,
`base_url`, default `model`, and relevant flags; the user only must paste a key.

| Preset key | label | driver | base_url | default model | flags |
|---|---|---|---|---|---|
| `anthropic` | Anthropic | `anthropic` | `https://api.anthropic.com` | `claude-opus-4-5` | thinking optional |
| `google` | Google (Gemini) | `openai` | `https://generativelanguage.googleapis.com/v1beta/openai/` | `gemini-2.5-pro` | — |
| `openai` | OpenAI | `openai-responses` | `https://api.openai.com/v1` | `gpt-5.2` | web_search available |
| `ollama` | Ollama (local) | `openai` (hardwired) | `http://localhost:11434/v1` | *(live-fetched)* | placeholder key `ollama` |
| `custom` | Custom | **auto-detect** (`openai`\|`anthropic`, default `openai` pre-Test) | (user-entered) | *(live-fetched / typed)* | — |

Numeric defaults (max_tokens, batch_dialogue_limit, timeout, connect_timeout,
concurrency) come from `get_default_config()`; model strings are indicative and
editable. Z.AI → Custom with base `https://api.z.ai/api/anthropic` (detects Anthropic);
OpenRouter → Custom with base `https://openrouter.ai/api/v1` (detects OpenAI).

**Curated model lists** (`config/presets.rs`) — per provider, easily updatable; the
live fetch (§5.7) is authoritative and supplements these:
- Anthropic: `claude-opus-4-5`, `claude-sonnet-4-5`, `claude-haiku-4-5` (+ recent 4.x).
- OpenAI: `gpt-5.2`, `gpt-5.1`, `gpt-5`, `gpt-4.1`, `o4-mini` (representative).
- Google: `gemini-2.5-pro`, `gemini-2.5-flash`, `gemini-2.0-flash`.
- Ollama: live only (curated examples like `llama3.1`, `qwen2.5` as a hint).

### 5.4 LLM driver port (`llm/`)
Trait:
```rust
#[async_trait]
trait LlmDriver {
    async fn complete(&self, system: &str, user: &str) -> Result<String, LlmError>;
    async fn list_models(&self) -> Result<Vec<String>, LlmError>;
    fn model(&self) -> &str;
}
```
Built on `reqwest` (async). Three impls mirror the Python request/response shapes
**exactly** (`llm/anthropic.py`, `llm/openai.py`, `llm/openai_responses.py`):

| Driver | Complete endpoint | Auth headers | Body (highlights) | Text extracted from | Models endpoint |
|---|---|---|---|---|---|
| Anthropic | `{base}/v1/messages` | `x-api-key`, `anthropic-version: 2023-06-01` | `model`, `max_tokens`, `system`, `messages:[{role:user,content}]`, *(opt)* `thinking` | `content[].{type=="text"}.text` | `GET {base}/v1/models` |
| OpenAI | `{base}/chat/completions` | `Authorization: Bearer` | `model`, `max_tokens`, `messages:[{system},{user}]` | `choices[0].message.content` | `GET {base}/models` |
| OpenAI-Responses | `{base}/responses` | `Authorization: Bearer` | `model`, `max_output_tokens`, `input:[{system},{user}]`, *(opt)* `tools:[web_search_preview]` | `output[].{type=="message"}.content[].{type=="output_text"}.text` | `GET {base}/models` |

`base_url` is right-trimmed of `/` before appending the path. Empty-content responses
become a clear `LlmError` (mirror the Python "Empty response from LLM" guards).
Models endpoints parse `data[].id`.

`create_driver(&Connection) -> Box<dyn LlmDriver>` matches on `driver` (mirrors
`client.py:_create_driver_from_settings`).

**`LlmError`** (`llm/error.rs`) — variants for transport, HTTP status (carry code +
body snippet), parse, and empty-response. `is_retryable()` mirrors `is_retryable_error`
(`llm/anthropic.py:30`): 401/403/404/auth → non-retryable; timeout / connection reset /
429 / 5xx → retryable. Surfaces a clean one-line message for the Test result. Folds
into `AppError`.

### 5.5 Commands (`commands/`)
All return `AppResult<…>` and derive ts-rs types for inputs/outputs.

| Command | Op | Behavior |
|---|---|---|
| `list_connections() -> ConnectionsView` | O1 | per-connection summaries (name, driver, `has_key`) + which is active + which is personalization. **Keys never included** here (only `read_connection` returns the key, for the editor). |
| `read_connection(name) -> Connection` | O2 | full settings for the editor |
| `save_connection(name, Connection)` | O3 | upsert into the store (unique name) |
| `delete_connection(name)` | O3 | remove; block removing the active one without reassigning |
| `set_active_connection(name)` | O4 | persist `active_connection` |
| `set_personalization_connection(name)` | O4 | persist `personalization_model` |
| `test_connection(Connection) -> TestResult` | O5 | build a driver from the **editor's** (possibly unsaved) fields; for Custom run **format detection** first (§5.7); send a tiny prompt; return ok + model + detected driver, or a classified error |
| `list_models(Connection) -> Vec<String>` | — | live `/models` fetch via the right driver; merged with curated list on the UI side |
| `first_run_status() -> FirstRunStatus` | O21 | `{ has_usable_connection: bool }` — true if any connection has a non-empty `api_key` |
| `list_presets() -> Vec<Preset>` | — | the preset table (incl. curated model lists) for the dropdown |

**Test request shape:** minimal and cheap — thinking **off**, web_search **off**, small
`max_tokens` (e.g. 16), short timeout. System ≈ "Reply with OK.", user ≈ "ping". It
only verifies auth + model reachability; success echoes the connection's model.
(Thinking stays off so a small `max_tokens` is valid.)

`TestResult = { ok: bool, model: String, detected_driver: Driver | null, message: String }`
(`detected_driver` populated for Custom; `message` = success copy or classified error).

### 5.6 Registration (`lib.rs`)
Add all commands to `invoke_handler!`. The store plugin is already registered.

### 5.7 API-format auto-detection (Custom) & model listing
**Ollama:** no detection — hardwired to the `openai` driver.

**Custom:** `detect_format(base_url, api_key) -> Driver` probes by **HTTP status code**,
which disambiguates the wire format *even when the key is invalid* (a route that exists
returns `200/400/401/403/429`; a route that doesn't exist returns `404` or a
connection error):

1. **OpenAI probe** — `POST {base}/chat/completions` with `Bearer` and a tiny valid body.
   - status ∈ {200, 400, 401, 403, 429} ⇒ **OpenAI-compatible** (route exists).
   - `404` / connection error ⇒ not OpenAI here; continue.
2. **Anthropic probe** — `POST {base}/v1/messages` with `x-api-key` + `anthropic-version`
   and a tiny valid body.
   - status ∈ {200, 400, 401, 403, 429} ⇒ **Anthropic-compatible**.
   - else ⇒ **undetermined** → return a clear "couldn't determine the API format at this
     URL" error.

`test_connection` for a Custom connection: run `detect_format` first, set the driver to
the result, then run the normal tiny `complete()` and report:
- detected + key valid → `ok=true`, `detected_driver`, "✓ Detected OpenAI-compatible —
  responded as {model}".
- detected + key rejected (401/403) → `ok=false`, `detected_driver` still set, message
  names the format and the auth failure (so the user knows the format was found).
- undetermined → `ok=false`, `detected_driver=null`, the undetermined-format message.

On **Save**, the detected `driver` is persisted so normal use doesn't re-probe.

**`list_models`** uses the (detected, for Custom) driver's models endpoint; results are
merged with the curated list and de-duplicated on the frontend. Failures are non-fatal
(fall back to curated + free typing).

---

## 6. Frontend design

### 6.1 Design tokens (`index.css`)
Replace the current generic shadcn neutral oklch values with the `theme.py` palette
(design-system doc §Color tokens): `bg_deepest #0f1114`, `bg_base #14161a`,
`bg_surface #1a1c20`, `bg_raised #1f2226`, `bg_hover #2a2d33`, `border #2d3036`,
`border_strong #3a3d44`, `text_muted #888`, `text_primary #d8d8d8`,
`text_emphasis #fff`, `accent #4a9eff`, `accent_deep #1f5fa6`, `alert #ffcc55`,
`danger #ff8a8a`, `success #8ad08a`, `state_cleanup #b48ead`, `state_verify #79c0c0`.
Map these onto the shadcn variable names (`--background`, `--card`, `--accent`,
`--destructive`, …) so existing maia components inherit the look. Dark is the primary
(and only required) theme. Spacing/radii/type per the design-system doc; counts/ETAs
use `tabular-nums`.

### 6.2 Shell
App grid: `[ rail | (header / main) ]` with a full-width status bar pinned bottom.
Replaces `AppLayout` + `MainNav`.

- **NavRail** (`components/nav-rail.tsx`): vertical icon+label list. Top group =
  workflow (Project=`folder`, Glossary=`book-open`, Translate=`play`,
  Verify=`check-circle`); divider; Connections=`lightning`; spacer; Settings=`gear`,
  Help=`question` (Phosphor names). Active = `bg_raised` + accent icon; disabled =
  `text_muted` with a gating tooltip. Per-item badge slot (Connections: `✓` tested /
  `⚠` none).
- **Header** (`components/page-header.tsx`, reworked): per-view title + (step 2) folder
  name/change + context chips. Step 1: minimal (view title + connection context chip).
- **StatusBar** (existing, reworked): left = folder ("No folder selected") + file/line
  counts (`—` in step 1); center = transient message slot; right = language pair
  (defaults), active-connection `StatusChip` (double-click → Connections), `StateChip`
  (Idle), ETA (hidden when idle). Keep `core vX` as a subtle right-end element.

### 6.3 Routing
Keep TanStack Router; the rail items are routes (rail style ≠ tabs — the "tabs are a
placeholder" note is about the old top-nav look, not the router).
- `/` (`routes/index.tsx`): on load, call `first_run_status`; if **not** usable,
  redirect to `/connections`; otherwise render a minimal placeholder **home** in place
  (full Welcome + folder pickup is step 2 — no self-redirect).
- `/connections` — real view.
- `/project`, `/glossary`, `/translate`, `/verify` — gated placeholders (redirect to a
  "open a folder first" state; full views in later steps).
- `/settings`, `/help` — thin `EmptyState` placeholders.

### 6.4 Connections view (`features/connections/`)
Two-pane (mirrors `connections-v1.html`, but with rail IA + `theme.py` tokens):
- **Left list:** each connection row with name + active dot; `＋ Add`. Selecting loads
  the editor (`read_connection`).
- **Right editor:**
  - **Provider** preset select (`list_presets`) — prefills driver/base_url/model/flags,
    never overwrites a typed key. Choosing **Custom** shows no format field (auto-detect).
  - **API key** `SetupField`: masked input + reveal toggle, **Test** button beside it,
    a result line (idle / testing / ✓ `responded as {model}` / ✓ `detected
    {format} — responded as {model}` for Custom / ✗ error), HelpText.
  - **Model** `SetupField`: **combobox** (cmdk `Command`, already a dep) showing curated
    ∪ live-fetched models (`list_models`), **free typing allowed** for unknown models.
    Live fetch triggers when a base_url (+ key where required) is present; failures fall
    back silently to curated + typing.
  - **Personalization checkbox**: "Use this connection for 'look up names online'" →
    `set_personalization_connection`. HelpText about needing a web-capable model.
  - **Advanced** collapse (`SectionHelp` pattern): base_url, max_tokens,
    batch_dialogue_limit, timeout, connect_timeout, concurrency, prompt_template,
    web_search, thinking_enabled, thinking_budget, thinking_glossary_norm_budget — each
    with HelpText. (For Custom, the detected driver shows here read-only after Test.)
  - **Footer:** `Remove` (left), `Save` + `Set as active` (right). Unsaved-dot optional.
- **First-run banner** when launched with no usable connection: "Welcome — let's
  connect an AI provider so you can start translating."
- Form via `react-hook-form` + `zod`; CRUD + Test + model fetch via TanStack Query
  mutations/queries; invalidate the connections query on save/delete/active changes.

### 6.5 State (`stores/app-store.ts`)
Extend with `activeConnection`, `personalizationConnection`, and a derived/queried
`hasUsableConnection`. Connection data itself stays server-state (TanStack Query over
the commands); the store holds only cross-view UI selections.

---

## 7. Data shapes crossing the seam (ts-rs)

New/updated `#[derive(TS)]` types exported to `src/types/generated/`:
- `Connection` (updated: + `prompt_template`, `thinking_glossary_norm_budget`)
- `AppConfig`, `Driver` (existing)
- `ConnectionsView { connections: ConnectionSummary[], active: string, personalization: string | null }`
  where `ConnectionSummary { name: string, driver: Driver, has_key: bool }` (no key)
- `Preset { key, label, driver: Driver | null, base_url, model, models: string[], flags… }`
  (`driver` null for Custom = auto-detect; `models` = curated list)
- `TestResult { ok, model, detected_driver: Driver | null, message }`
- `FirstRunStatus { has_usable_connection }`
- `list_models` returns `string[]`.

Add `bun gen:bindings` / `bun gen:routes` scripts to `package.json` (README references
them but they're missing) — `gen:bindings` runs the ts-rs export (cargo test), and
verify `src/types/generated/` after Rust type changes.

---

## 8. Testing strategy

**Rust (unit, no network):**
- Default seeding produces the 4 seeded connections + correct active/personalization.
- `ConfigStore` load/save round-trips an `AppConfig` (temp store / injected backend).
- Each driver builds the correct URL, headers, and JSON body (assert on the serialized
  request) and parses text from captured response fixtures for all three shapes;
  `list_models` parses `data[].id`.
- `LlmError::is_retryable()` parity table vs. the Python `is_retryable_error` patterns.
- **Format detection** (`detect_format`) against a mock server: OpenAI-only base,
  Anthropic-only base, both-404 (undetermined), and the 401-but-route-exists case
  (asserts it still resolves the format).
- `test_connection` happy path + auth-error + Custom-detect paths against a **mock HTTP
  server** (`mockito` or `wiremock`). No real provider calls in CI.

**Toolchain:** `cargo build`, ts-rs binding gen, `bun run build` all green.

**Manual:** one real Test against a provider with a valid key (out-of-band; not in CI).

---

## 9. Risks & follow-ups

- **Auto-detect robustness:** status-code disambiguation (not just 200) is what makes
  "auto-detect only" workable with a bad key. The genuinely undetermined cases (both
  routes 404, or the host unreachable) must surface a clear, actionable message — don't
  silently pick a format.
- **`@phosphor-icons/react` migration:** existing `main-nav`/`status-bar` import
  `@hugeicons/*`. Those components get reworked anyway; migrate icon usages as we touch
  them.
- **Tauri store Rust API:** confirm `StoreExt`/`app.store(...)` usage for the installed
  `tauri-plugin-store` version; the existing generated types suggest the plugin is
  wired, but Rust-side read/write must be verified early.
- **`reqwest` in `src-tauri/Cargo.toml`:** `error.rs` already references
  `reqwest::Error`, so it's a dep — confirm features (`json`, TLS) and that `tokio` +
  `async-trait` + a mock-HTTP dev-dep are available.
- **Model IDs / Gemini base:** curated model IDs and the Gemini base
  (`…/v1beta/openai/`, `gemini-2.5-pro`) are best-effort — verify against current
  provider docs during implementation; the live fetch is the safety net.
- **Test cost/latency:** keep the Test request minimal (16 max_tokens, thinking off).

---

## 10. Python source-of-truth map

| Concern | Python reference |
|---|---|
| Defaults / presets | `config/settings.py:get_default_config` (`:16`) |
| Connection fields | `config/settings.py:LlmConnectionSettings` (`:186`) |
| Read/active/persist | `Settings.get_connection` (`:223`), `update_setting` (`:336`) |
| Driver factory | `llm/client.py:create_llm_driver` (`:14`), `_create_driver_from_settings` (`:33`) |
| Anthropic shape | `llm/anthropic.py:AnthropicDriver.complete` (`:104`) |
| OpenAI shape | `llm/openai.py:OpenAiDriver.complete` (`:26`) |
| OpenAI-Responses shape | `llm/openai_responses.py:OpenAiResponsesDriver.complete` (`:31`) |
| Error classification | `llm/anthropic.py:is_retryable_error` (`:30`) |
| Test (O5) | a minimal `LlmClient.complete()` via the right driver |
| First-run (O21) | inspect `Settings.connections` for a usable `api_key` |

UI references: `polygluttony-docs/windows/00-shell-rail-statusbar.md`,
`windows/02-connections.md`, `01-design-system.md`, `03-operations-and-flows.md`
(O1–O5, O20-subset, O21; gating table).
