<p align="center">
  <a href="https://github.com/nearai/ironclaw"><img src="./doc/ironclaw-logo.png" width="90" alt="Ironclaw" style="vertical-align: middle"></a>
  &nbsp;&nbsp;&nbsp;&nbsp;
  <a href="https://cursor.sh"><img src="./doc/cursor-logo.svg" width="80" alt="Cursor" style="vertical-align: middle"></a>
</p>

<h1 align="center">ironclaw-cursor-brain</h1>

<p align="center">
  将 <a href="https://cursor.sh">Cursor</a> 作为 OpenAI 兼容的 LLM 后端接入 <a href="https://github.com/nearai/ironclaw">Ironclaw</a>。
</p>

<p align="center">
  <a href="README.md">English</a>
  &nbsp;·&nbsp;
  <a href="https://github.com/nearai/ironclaw">Ironclaw</a>
  &nbsp;·&nbsp;
  <a href="https://www.rust-lang.org"><img src="https://img.shields.io/badge/Rust-1.70%2B-orange?logo=rust" alt="Rust"></a>
  &nbsp;·&nbsp;
  <img src="https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-blue" alt="Platform">
</p>

---

**ironclaw-cursor-brain** 是 [Ironclaw](https://github.com/nearai/ironclaw) 的 OpenAiCompletions provider，将 Cursor Agent（子进程）封装为符合 OpenAI Chat Completions 的 HTTP 服务。在 `~/.ironclaw/providers.json` 中增加一条配置即可像内置 provider（groq、openai）一样使用，无需修改 Ironclaw 源码。

## 特性

- **OpenAI 兼容 API** — `POST /v1/chat/completions`，支持流式（SSE）与非流式
- **会话延续** — 相同 `X-Session-Id` 可恢复 Cursor 对话；映射持久化在 `~/.ironclaw/` 下
- **默认零配置** — 可选 `~/.ironclaw/cursor-brain.json`；环境变量可覆盖
- **跨平台** — Windows、macOS、Linux（Rust + cursor-agent）

## 目录

- [快速开始](#快速开始)
- [安装](#安装)
- [配置](#配置)
- [运行与验证](#运行与验证)
- [注册为 Ironclaw provider](#如何成为-ironclaw-的-provider)
- [会话延续](#会话延续可选)
- [API](#api)
- [许可与参考](#许可与参考)

## 快速开始

若已安装 [Rust](https://rustup.rs)、[Cursor](https://cursor.com)（提供 cursor-agent）和 [Ironclaw](https://github.com/nearai/ironclaw)：

```bash
git clone https://github.com/nearai/ironclaw-cursor-brain.git && cd ironclaw-cursor-brain
cargo build --release
./target/release/ironclaw-cursor-brain   # 或 cargo run
```

**Windows** 下在项目目录执行 `.\target\release\ironclaw-cursor-brain.exe` 或 `cargo run`。

在 `~/.ironclaw/providers.json`（Windows 下为 `%USERPROFILE%\.ironclaw\providers.json`）中添加 [provider 条目](#如何成为-ironclaw-的-provider)，即可在 Ironclaw 中使用 Cursor 后端。

## 安装

**环境要求：** [Rust](https://rustup.rs)（stable）、[cursor-agent](https://cursor.com)（随 Cursor 或 PATH）。完整使用需按顺序安装：PostgreSQL 15+ 与 pgvector、Ironclaw、本插件。以下步骤覆盖 Windows、macOS 与 Linux。

### PostgreSQL 15+ 与 pgvector

Ironclaw 需要 PostgreSQL 15+ 与 [pgvector](https://github.com/pgvector/pgvector) 扩展。

- **Windows**：安装 [PostgreSQL](https://www.postgresql.org/download/windows/) 15+（如 EDB 安装包）。再安装 pgvector：确保 Visual Studio 生成工具与 PostgreSQL 的 `pg_config`（在 bin 目录）在 PATH 中，克隆 [pgvector](https://github.com/pgvector/pgvector)，在该目录执行 `nmake /F Makefile.win` 与 `nmake /F Makefile.win install`，然后重启 PostgreSQL。
- **macOS**：`brew install postgresql@15`（或 `postgresql` 使用最新版）。安装 pgvector：若有则 `brew install pgvector`，否则克隆 [pgvector](https://github.com/pgvector/pgvector) 后执行 `make && make install`（确保 `pg_config` 在 PATH）。启动 PostgreSQL（如 `brew services start postgresql@15`）。
- **Linux**：通过发行版安装 PostgreSQL 15+（如 Debian/Ubuntu 上 `sudo apt install postgresql-15`，或见 [PostgreSQL 官方](https://www.postgresql.org/download/linux/)）。再克隆 [pgvector](https://github.com/pgvector/pgvector) 并执行 `make && sudo make install`。启动 PostgreSQL 服务。

**一次性数据库设置**（供 Ironclaw 使用）：

```bash
createdb ironclaw
psql ironclaw -c "CREATE EXTENSION IF NOT EXISTS vector;"
```

更多见 [Ironclaw README](https://github.com/nearai/ironclaw/blob/staging/README.zh-CN.md)。

### Ironclaw

- **Windows**：下载 [Windows 安装包 (MSI)](https://github.com/nearai/ironclaw/releases/latest/download/ironclaw-x86_64-pc-windows-msvc.msi) 并运行，或使用 PowerShell 脚本：`irm https://github.com/nearai/ironclaw/releases/latest/download/ironclaw-installer.ps1 | iex`
- **macOS / Linux**：运行 shell 安装脚本：`curl --proto '=https' --tlsv1.2 -LsSf https://github.com/nearai/ironclaw/releases/latest/download/ironclaw-installer.sh | sh`，或 Homebrew：`brew install ironclaw`。也可克隆 [Ironclaw 仓库](https://github.com/nearai/ironclaw) 后执行 `cargo build --release`。

然后运行 `ironclaw onboard` 配置数据库与认证。详见 [Ironclaw Releases](https://github.com/nearai/ironclaw/releases) 与 [README](https://github.com/nearai/ironclaw/blob/staging/README.zh-CN.md)。

### 本插件（ironclaw-cursor-brain）

- **所有平台**：安装 [Rust](https://rustup.rs)（`rustup`）。克隆本仓库并编译：`git clone <本仓库地址> && cd ironclaw-cursor-brain && cargo build --release`。可执行文件在 `target/release/ironclaw-cursor-brain`（Windows 下为 `ironclaw-cursor-brain.exe`）。确保 **cursor-agent** 可用（安装 [Cursor](https://cursor.com) 或将 agent 可执行文件加入 PATH）。可选：将生成的二进制加入 PATH 或在项目目录下运行。

## 配置

**插件配置复用 Ironclaw 的目录与接口：** 所有配置均在与 Ironclaw 相同的基础目录下（与 Ironclaw 使用相同解析方式：**dirs** 库 — macOS/Linux 下即 `~/.ironclaw/`，Windows 下为用户配置目录）。可选插件配置文件：该目录下的 `cursor-brain.json`。Provider 注册通过同目录下的 `providers.json` 完成（与 Ironclaw 内置 provider 相同）。运行 `ironclaw onboard` 后，在该文件中添加本插件的 provider 条目（见下方「如何成为 Ironclaw 的 provider」）。

- **来源**：环境变量优先；可选文件 `~/.ironclaw/cursor-brain.json`（环境变量覆盖文件）
- **项**：
  - `cursor_path`：cursor-agent 可执行路径；不设则从 PATH 或平台固定路径探测
  - `port`：监听端口，默认 **3001**（Ironclaw 生态约定：Web Gateway 3000 + 1）
  - `request_timeout_sec`：单次请求超时（秒），默认 300
  - `session_cache_max`：会话映射 LRU 容量，默认 1000
  - `session_header_name`：请求头中用于传递外部 session id 的名称，默认 `x-session-id`（如 `X-Session-Id`）

环境变量示例：`CURSOR_PATH`、`PORT` 或 `IRONCLAW_CURSOR_BRAIN_PORT`、`REQUEST_TIMEOUT_SEC`、`SESSION_CACHE_MAX`、`SESSION_HEADER_NAME`。

会话映射始终持久化到 `~/.ironclaw/cursor-brain-sessions.json`（固定路径，不可配置）。

## 运行与验证

```bash
cargo run   # 默认监听 http://0.0.0.0:3001
```

在另一终端：

```bash
curl http://127.0.0.1:3001/v1/health
curl -X POST http://127.0.0.1:3001/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{"model":"cursor-default","messages":[{"role":"user","content":"hi"}],"stream":false}'
```

## 如何成为 Ironclaw 的 provider

在 **`~/.ironclaw/providers.json`** 中增加一条 **ProviderDefinition**（与 Ironclaw 内置 provider 同表、同协议）。常用字段：

| 字段             | 说明                   | 示例                         |
| ---------------- | ---------------------- | ---------------------------- |
| id               | 唯一标识               | `"cursor"`                   |
| protocol         | 协议                   | `"OpenAiCompletions"`        |
| default_base_url | 本服务地址（需含 /v1） | `"http://127.0.0.1:3001/v1"` |
| default_model    | 默认模型               | `"auto"`                     |
| description      | 描述                   | `"Cursor Agent (local)"`     |

**示例**（合并进 `~/.ironclaw/providers.json` 的数组中）：

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

完成后，Ironclaw 会像使用其他 OpenAiCompletions provider 一样使用本服务；默认端口 3001 为 Ironclaw 生态约定，可与 Web Gateway（3000）区分。

## 会话延续（可选）

请求时在 HTTP 头中带上配置的 session 头（默认 `X-Session-Id`）并传同一值，本服务会维护「外部 session id → cursor session_id」映射（进程内 LRU，容量可配置）。下次同 session 请求将使用 `--resume` 延续对话。若本次使用了 resume 但返回内容为空，会清除该映射并自动重试一次（不传 resume）。

映射会持久化到 `~/.ironclaw/cursor-brain-sessions.json`（每次写盘采用临时文件 + rename）。重启后保留该文件即可延续会话。

## API

| 端点                        | 说明                                                            |
| --------------------------- | --------------------------------------------------------------- |
| `POST /v1/chat/completions` | OpenAI 格式请求体；支持 `stream: true`（SSE）与 `stream: false` |
| `GET /v1/models`            | 模型列表（最小兼容）                                            |
| `GET /v1/health`            | 健康与 cursor 可用性                                            |

请求体中的 `temperature`、`max_tokens` 会解析但不转发给 cursor-agent（使用其默认值）。

## 许可与参考

- 实现参考：[openclaw-cursor-brain](https://github.com/openclaw/openclaw-cursor-brain)（cursor-agent 封装、stream-json）。
- 与 Ironclaw 的集成仅通过 provider 注册契约，不引入 OpenClaw 概念。
