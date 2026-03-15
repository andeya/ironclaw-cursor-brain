<p align="center">
  <a href="https://github.com/nearai/ironclaw"><img src="./doc/ironclaw-logo.png" width="90" alt="Ironclaw" style="vertical-align: middle"></a>
  &nbsp;&nbsp;&nbsp;&nbsp;
  <a href="https://cursor.sh"><img src="./doc/cursor-logo.svg" width="80" alt="Cursor" style="vertical-align: middle"></a>
</p>

<h1 align="center">ironclaw-cursor-brain</h1>

<p align="center">
  Use <a href="https://cursor.sh">Cursor</a> as an OpenAI-compatible LLM backend for <a href="https://github.com/nearai/ironclaw">Ironclaw</a>.
</p>

<p align="center">
  <a href="README.zh-CN.md">简体中文</a>
  &nbsp;·&nbsp;
  <a href="https://github.com/nearai/ironclaw">Ironclaw</a>
  &nbsp;·&nbsp;
  <a href="https://www.rust-lang.org"><img src="https://img.shields.io/badge/Rust-1.70%2B-orange?logo=rust" alt="Rust"></a>
  &nbsp;·&nbsp;
  <img src="https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-blue" alt="Platform">
</p>

---

**ironclaw-cursor-brain** is an OpenAiCompletions provider for [Ironclaw](https://github.com/nearai/ironclaw). It wraps the Cursor Agent (subprocess) as an OpenAI Chat Completions–compatible HTTP service. Add one entry to `~/.ironclaw/providers.json` and use it like built-in providers (groq, openai) — no Ironclaw source changes.

## Features

- **OpenAI-compatible API** — `POST /v1/chat/completions` with streaming (SSE) and non-streaming
- **Session continuity** — Same `X-Session-Id` resumes Cursor conversations; mappings persisted under `~/.ironclaw/`
- **Zero config by default** — Optional `~/.ironclaw/cursor-brain.json`; env vars override
- **Cross-platform** — Windows, macOS, Linux (Rust + cursor-agent)

## Table of contents

- [Quick start](#quick-start)
- [Installation](#installation)
- [Configuration](#configuration)
- [Run & validate](#run--validate)
- [Register as Ironclaw provider](#register-as-an-ironclaw-provider)
- [Session continuity](#session-continuity-optional)
- [API](#api)
- [License & references](#license-and-references)

## Quick start

If you already have [Rust](https://rustup.rs), [Cursor](https://cursor.com) (for cursor-agent), and [Ironclaw](https://github.com/nearai/ironclaw) set up:

```bash
git clone https://github.com/nearai/ironclaw-cursor-brain.git && cd ironclaw-cursor-brain
cargo build --release
./target/release/ironclaw-cursor-brain   # or cargo run
```

On **Windows**, run `.\target\release\ironclaw-cursor-brain.exe` or `cargo run` from the project directory.

Add a [provider entry](#register-as-an-ironclaw-provider) to `~/.ironclaw/providers.json` (or `%USERPROFILE%\.ironclaw\providers.json` on Windows), then use the Cursor backend from Ironclaw.

## Installation

**Requirements:** [Rust](https://rustup.rs) (stable), [cursor-agent](https://cursor.com) (from Cursor or PATH). For the full stack, install in order: PostgreSQL 15+ with pgvector, Ironclaw, then this plugin. Steps below cover Windows, macOS, and Linux.

### PostgreSQL 15+ and pgvector

Ironclaw requires PostgreSQL 15+ and the [pgvector](https://github.com/pgvector/pgvector) extension.

- **Windows**: Install [PostgreSQL](https://www.postgresql.org/download/windows/) 15+ (e.g. EDB installer). Then install pgvector: ensure Visual Studio Build Tools and `pg_config` (from the PostgreSQL bin directory) are on PATH, clone [pgvector](https://github.com/pgvector/pgvector), then run `nmake /F Makefile.win` and `nmake /F Makefile.win install` in the pgvector directory. Restart PostgreSQL.
- **macOS**: `brew install postgresql@15` (or `postgresql` for latest). Install pgvector: `brew install pgvector` if available, or clone [pgvector](https://github.com/pgvector/pgvector) and run `make && make install` (ensure `pg_config` is on PATH). Start PostgreSQL (e.g. `brew services start postgresql@15`).
- **Linux**: Install PostgreSQL 15+ via your distro (e.g. `sudo apt install postgresql-15` on Debian/Ubuntu, or [PostgreSQL docs](https://www.postgresql.org/download/linux/)). Then clone [pgvector](https://github.com/pgvector/pgvector) and run `make && sudo make install`. Start the PostgreSQL service.

**One-time database setup** (for Ironclaw):

```bash
createdb ironclaw
psql ironclaw -c "CREATE EXTENSION IF NOT EXISTS vector;"
```

See [Ironclaw README](https://github.com/nearai/ironclaw/blob/staging/README.zh-CN.md) for more detail.

### Ironclaw

- **Windows**: Download the [Windows installer (MSI)](https://github.com/nearai/ironclaw/releases/latest/download/ironclaw-x86_64-pc-windows-msvc.msi) and run it, or use the PowerShell script: `irm https://github.com/nearai/ironclaw/releases/latest/download/ironclaw-installer.ps1 | iex`
- **macOS / Linux**: Run the shell installer: `curl --proto '=https' --tlsv1.2 -LsSf https://github.com/nearai/ironclaw/releases/latest/download/ironclaw-installer.sh | sh`, or install via Homebrew: `brew install ironclaw`. Alternatively, clone the [Ironclaw repo](https://github.com/nearai/ironclaw) and run `cargo build --release`.

Then run `ironclaw onboard` to configure database and auth. See [Ironclaw Releases](https://github.com/nearai/ironclaw/releases) and [README](https://github.com/nearai/ironclaw/blob/staging/README.zh-CN.md).

### This plugin (ironclaw-cursor-brain)

- **All platforms**: Install [Rust](https://rustup.rs) (`rustup`). Clone this repo and build: `git clone <this-repo-url> && cd ironclaw-cursor-brain && cargo build --release`. The binary is at `target/release/ironclaw-cursor-brain` (or `ironclaw-cursor-brain.exe` on Windows). Ensure **cursor-agent** is available (install [Cursor](https://cursor.com) or put the agent binary on PATH). Optional: put the binary on PATH or run from the project directory.

## Configuration

**Plugin configuration reuses Ironclaw’s layout:** all config lives under the same base directory as Ironclaw (same resolution as Ironclaw: **dirs** crate — `~/.ironclaw/` on macOS/Linux, user profile on Windows). Optional plugin config file: `cursor-brain.json` in that directory. Provider registration is done via `providers.json` in the same directory (same as Ironclaw’s built-in providers). After running `ironclaw onboard`, add this plugin’s provider entry to that file (see “Register as an Ironclaw provider” below).

- **Source**: Environment variables first; optional file `~/.ironclaw/cursor-brain.json` (env overrides file).
- **Options**:
  - `cursor_path`: Path to cursor-agent; unset = detect from PATH or platform paths
  - `port`: Listen port, default **3001** (Ironclaw convention: Web Gateway 3000 + 1)
  - `request_timeout_sec`: Per-request timeout (seconds), default 300
  - `session_cache_max`: Session mapping LRU capacity, default 1000
  - `session_header_name`: HTTP header name for external session id, default `x-session-id` (e.g. `X-Session-Id`)

Env vars: `CURSOR_PATH`, `PORT` or `IRONCLAW_CURSOR_BRAIN_PORT`, `REQUEST_TIMEOUT_SEC`, `SESSION_CACHE_MAX`, `SESSION_HEADER_NAME`.

Session mappings are always persisted to `~/.ironclaw/cursor-brain-sessions.json` (fixed path, not configurable).

## Run & validate

```bash
cargo run   # listens on http://0.0.0.0:3001
```

In another terminal:

```bash
curl http://127.0.0.1:3001/v1/health
curl -X POST http://127.0.0.1:3001/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{"model":"cursor-default","messages":[{"role":"user","content":"hi"}],"stream":false}'
```

## Register as an Ironclaw provider

Add one **ProviderDefinition** to **`~/.ironclaw/providers.json`** (same table and protocol as Ironclaw’s built-in providers). Common fields:

| Field            | Description               | Example                      |
| ---------------- | ------------------------- | ---------------------------- |
| id               | Unique id                 | `"cursor"`                   |
| protocol         | Protocol                  | `"OpenAiCompletions"`        |
| default_base_url | Service URL (include /v1) | `"http://127.0.0.1:3001/v1"` |
| default_model    | Default model             | `"auto"`                     |
| description      | Description               | `"Cursor Agent (local)"`     |

**Example** (merge into the array in `~/.ironclaw/providers.json`):

```json
{
  "id": "cursor",
  "aliases": ["cursor-brain"],
  "protocol": "OpenAiCompletions",
  "default_base_url": "http://127.0.0.1:3001/v1",
  "base_url_required": false,
  "api_key_required": false,
  "default_model": "auto",
  "description": "Cursor Agent (local proxy)"
}
```

After that, Ironclaw uses this service like other OpenAiCompletions providers; default port 3001 follows the Ironclaw convention (Web Gateway 3000 + 1).

## Session continuity (optional)

Send the configured session header (default `X-Session-Id`) with the same value on each request; the service keeps an "external session id → cursor session_id" mapping (in-process LRU, capacity configurable). The next request with the same session uses `--resume` to continue the conversation. If a resume returns no content, the mapping is cleared and one retry without resume is done.

The mapping is persisted to `~/.ironclaw/cursor-brain-sessions.json` (temp file + rename on each write). Keep that file across restarts to preserve sessions.

## API

| Endpoint                    | Description                                                          |
| --------------------------- | -------------------------------------------------------------------- |
| `POST /v1/chat/completions` | OpenAI-style body; supports `stream: true` (SSE) and `stream: false` |
| `GET /v1/models`            | Model list (minimal compatibility)                                   |
| `GET /v1/health`            | Health and cursor availability                                       |

`temperature` and `max_tokens` are parsed but not forwarded to cursor-agent (it uses its own defaults).

## License and references

- Implementation reference: [openclaw-cursor-brain](https://github.com/openclaw/openclaw-cursor-brain) (cursor-agent wrapping, stream-json).
- Integration with Ironclaw is via the provider registry only; no OpenClaw concepts.
