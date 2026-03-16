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
  <a href="https://crates.io/crates/ironclaw-cursor-brain"><img src="https://img.shields.io/crates/v/ironclaw-cursor-brain" alt="crates.io"></a>
  &nbsp;·&nbsp;
  <a href="https://crates.io/crates/ironclaw-cursor-brain"><img src="https://img.shields.io/crates/d/ironclaw-cursor-brain" alt="downloads"></a>
  &nbsp;·&nbsp;
  <img src="https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-blue" alt="Platform">
  &nbsp;·&nbsp;
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-green" alt="License: MIT"></a>
</p>

---

**ironclaw-cursor-brain** 是 [Ironclaw](https://github.com/nearai/ironclaw) 的 OpenAiCompletions provider，将 Cursor Agent（子进程）封装为符合 OpenAI Chat Completions 的 HTTP 服务。在 `~/.ironclaw/providers.json` 中增加一条配置即可像内置 provider（groq、openai）一样使用，无需修改 Ironclaw 源码。

## 特性

- **OpenAI 兼容 API** — `POST /v1/chat/completions`，支持流式（SSE）与非流式
- **会话延续** — 相同 `X-Session-Id` 可恢复 Cursor 对话；映射持久化在 `~/.ironclaw/` 下
- **默认零配置** — 可选 `~/.ironclaw/cursor-brain.json`；环境变量可覆盖
- **跨平台** — Windows、macOS、Linux（Rust + cursor-agent）

**与 OpenAiCompletions 的对应：** 插件将**完整对话**以单条合成 prompt（System / User / Assistant / Tool result 段落）发给 cursor-agent。cursor-agent 仅支持单条 prompt（stdin）；请求中的 `tools` / `tool_choice` 会接收但不转发。详见 [doc/ironclaw-provider-contract.md](doc/ironclaw-provider-contract.md)。

## 目录

- [快速开始](#快速开始)
- [技术文档](#技术文档)
- [安装](#安装)
- [配置](#配置)
- [运行与验证](#运行与验证)
- [注册为 Ironclaw provider](#如何成为-ironclaw-的-provider)
- [会话延续](#会话延续可选)
- [API](#api)
- [许可与参考](#许可与参考)

## 技术文档

- **[架构说明](doc/ARCHITECTURE.zh-CN.md)** — 组件概览、请求流程、模块职责（含 Mermaid 图）。
- **[Ironclaw 提供方契约](doc/ironclaw-provider-contract.md)** — Ironclaw 如何调用插件；请求/响应契约。
- **[Provider 定义](doc/provider-definition.json)** — 用于 `~/.ironclaw/providers.json` 的 JSON 条目。

## 快速开始

若已安装 [Rust](https://rustup.rs)、[Cursor](https://cursor.com)（提供 cursor-agent）和 [Ironclaw](https://github.com/nearai/ironclaw)：

```bash
cargo install ironclaw-cursor-brain
ironclaw-cursor-brain
```

在 `~/.ironclaw/providers.json`（Windows 下为 `%USERPROFILE%\.ironclaw\providers.json`）中添加 [provider 条目](#如何成为-ironclaw-的-provider)，即可在 Ironclaw 中使用 Cursor 后端。

## 安装

**环境要求：** [Rust](https://rustup.rs)（stable）、[cursor-agent](https://cursor.com)（随 Cursor 或 PATH）。完整使用需按顺序安装：PostgreSQL 15+ 与 pgvector、Ironclaw、本插件。以下步骤覆盖 Windows、macOS 与 Linux。

### PostgreSQL 15+ 与 pgvector

Ironclaw 需要 PostgreSQL 15+ 与 [pgvector](https://github.com/pgvector/pgvector) 扩展。

- **Windows**：安装 [PostgreSQL](https://www.postgresql.org/download/windows/) 15+（如 EDB 安装包）。再安装 pgvector：确保 Visual Studio 生成工具与 PostgreSQL 的 `pg_config`（在 bin 目录）在 PATH 中，克隆 [pgvector](https://github.com/pgvector/pgvector)，在该目录执行 `nmake /F Makefile.win` 与 `nmake /F Makefile.win install`，然后重启 PostgreSQL。
- **macOS**：`brew install postgrest@15`（或从 https://postgresapp.com/downloads.html 下载App）。安装 pgvector：若有则 `brew install pgvector`，否则克隆 [pgvector](https://github.com/pgvector/pgvector) 后执行 `make && make install`（确保 `pg_config` 在 PATH）。启动 PostgreSQL（如 `brew services start postgresql@15`）。
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

- **所有平台**：从 [crates.io](https://crates.io/crates/ironclaw-cursor-brain) 安装：`cargo install ironclaw-cursor-brain`。二进制会安装到 Cargo 的 `bin` 目录（一般在 PATH 中）。确保 **cursor-agent** 可用（安装 [Cursor](https://cursor.com) 或将 agent 加入 PATH）。从源码编译：`git clone https://github.com/nearai/ironclaw-cursor-brain.git && cd ironclaw-cursor-brain && cargo build --release`。

## 配置

**插件配置复用 Ironclaw 的目录与接口：** 所有配置均在与 Ironclaw 相同的基础目录下（与 Ironclaw 使用相同解析方式：**dirs** 库 — macOS/Linux 下即 `~/.ironclaw/`，Windows 下为用户配置目录）。可选插件配置文件：该目录下的 `cursor-brain.json`。Provider 注册通过同目录下的 `providers.json` 完成（与 Ironclaw 内置 provider 相同）。运行 `ironclaw onboard` 后，在该文件中添加本插件的 provider 条目（见下方「如何成为 Ironclaw 的 provider」）。

- **来源**：环境变量优先；可选文件 `~/.ironclaw/cursor-brain.json`（环境变量覆盖文件）
- **项**：
  - `cursor_path`：cursor-agent 可执行路径；不设则从 PATH 或平台固定路径探测
  - `port`：监听端口，默认 **3001**（Ironclaw 生态约定：Web Gateway 3000 + 1）
  - `request_timeout_sec`：单次请求超时（秒），默认 300
  - `session_cache_max`：会话映射 LRU 容量，默认 1000
  - `session_header_name`：请求头中用于传递外部 session id 的名称，默认 `x-session-id`（如 `X-Session-Id`）
  - `default_model`：请求未传 `model` 时使用的默认模型，不设则为 `"auto"`
  - `fallback_model`：主模型返回空内容时，用此模型再重试一次（仅非流式）

环境变量示例：`CURSOR_PATH`、`PORT` 或 `IRONCLAW_CURSOR_BRAIN_PORT`、`REQUEST_TIMEOUT_SEC`、`SESSION_CACHE_MAX`、`SESSION_HEADER_NAME`、`CURSOR_BRAIN_DEFAULT_MODEL`、`CURSOR_BRAIN_FALLBACK_MODEL`。

- **日志级别**：通过 **`RUST_LOG`** 控制（未设置时默认为 `info`）。例如：`RUST_LOG=debug ironclaw-cursor-brain` 或 `RUST_LOG=ironclaw_cursor_brain=debug,tower_http=info` 只提高本服务的 debug。可选级别：`error`、`warn`、`info`、`debug`、`trace`。

会话映射始终持久化到 `~/.ironclaw/cursor-brain-sessions.json`（固定路径，不可配置）。

## 运行与验证

```bash
ironclaw-cursor-brain   # 默认监听 http://0.0.0.0:3001（从源码编译则可用 cargo run）
```

在另一终端：

```bash
curl http://127.0.0.1:3001/v1/health
curl -X POST http://127.0.0.1:3001/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{"model":"cursor-default","messages":[{"role":"user","content":"hi"}],"stream":false}'
```

**出现 503「cursor-agent returned no content」时：** 插件会记录 cursor-agent 的 stderr 到日志（`cursor_agent_stderr`）。常见原因：请求里的 `model` 为 `cursor` 时，插件已自动改为 `auto` 传给 cursor-agent；若仍报错请查看插件控制台/日志。若 stderr 为空，可增大 `request_timeout_sec` 或手动运行 `cursor-agent -p --output-format stream-json` 测试。**若 Ironclaw 报「Failed to parse user providers.json」**：请将 `protocol` 设为 `"open_ai_completions"`（蛇形命名），不要用 `OpenAiCompletions`。

## 如何成为 Ironclaw 的 provider

Ironclaw **没有单独的 manifest 或安装包**：第三方 provider 的注册方式只有一种——在 **`~/.ironclaw/providers.json`** 的 JSON 数组里加入一条完整的 **ProviderDefinition**（与内置 provider 同结构）。该文件与 Ironclaw 内置的 `providers.json` 会在加载时合并，用户文件中的条目会追加并参与去重。

- **怎样才会出现在配置流程里？** 定义里必须带 **`setup`** 字段。Ironclaw 的配置向导只展示「带 `setup` 的 provider」；没有 `setup` 的条目不会出现在「选择 LLM 提供方」的列表里。
- **`setup.can_list_models: true` 的作用**：在向导的「选择模型」步骤中，Ironclaw 会请求该 provider 的 **GET /v1/models**（用 `default_base_url`），把返回的模型列表展示给用户选择；本插件已实现该接口（通过 `cursor-agent --list-models` 查询），故此处应设为 `true`。
- **谁设置、存哪儿？** 由**用户**在本地编辑 `~/.ironclaw/providers.json` 添加本条；仅存在该文件中，无其他 manifest 或存储位置。

**若该文件不存在**（例如刚完成 `ironclaw onboard` 尚未添加过 provider），请先新建文件，内容为 `[]`，再将下方完整条目**合并**进该数组（不要漏掉必填字段 `model_env` 和 **`setup`**，否则无法出现在向导中或解析失败）。完成后在 `ironclaw onboard` 的 LLM 步骤中即可选择 **Cursor Brain**，并可在模型步骤中从 cursor-agent 的模型列表里选择。

### 字段说明

| 字段              | 必填 | 说明                                                                                                                                                                     |
| ----------------- | ---- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| id                | ✓    | 唯一标识，用于 `LLM_BACKEND`，如 `"cursor"`                                                                                                                              |
| protocol          | ✓    | 固定为 `"open_ai_completions"`（蛇形命名）                                                                                                                               |
| model_env         | ✓    | 模型环境变量名，如 `"CURSOR_BRAIN_MODEL"`；Ironclaw 解析配置时必读                                                                                                       |
| default_model     | ✓    | 默认模型，推荐 `"auto"`（cursor-agent 接受）                                                                                                                             |
| description       | ✓    | 一行描述，用于界面展示                                                                                                                                                   |
| aliases           |      | 别名列表，如 `["cursor_brain","cursor-brain"]`，便于 `LLM_BACKEND=cursor_brain`                                                                                          |
| default_base_url  |      | 默认服务地址（需含 `/v1`），推荐 `"http://127.0.0.1:3001/v1"`                                                                                                            |
| base_url_env      |      | 覆盖 base URL 的环境变量，如 `"CURSOR_BRAIN_BASE_URL"`                                                                                                                   |
| base_url_required |      | 是否强制要求 base URL，本插件填 `false`                                                                                                                                  |
| api_key_required  |      | 是否强制 API Key，本插件填 `false`                                                                                                                                       |
| setup             |      | **必填才可被安装**。配置向导提示；`kind: "open_ai_compatible"` 使向导显示「Cursor Brain」并询问 Base URL；`can_list_models: true` 使向导请求 GET /v1/models 供用户选模型 |

### 完整示例（推荐直接使用）

可直接复制 [doc/provider-definition.json](doc/provider-definition.json) 中的对象，合并进你现有的 `~/.ironclaw/providers.json` 数组；或使用下面完整 JSON（若文件已存在则只取 `{ ... }` 合并进数组）：

```json
{
  "id": "cursor",
  "aliases": ["cursor_brain", "cursor-brain"],
  "protocol": "open_ai_completions",
  "default_base_url": "http://127.0.0.1:3001/v1",
  "base_url_env": "CURSOR_BRAIN_BASE_URL",
  "base_url_required": false,
  "api_key_required": false,
  "model_env": "CURSOR_BRAIN_MODEL",
  "default_model": "auto",
  "description": "Cursor Agent via ironclaw-cursor-brain (local OpenAI-compatible proxy)",
  "setup": {
    "kind": "open_ai_compatible",
    "secret_name": "llm_cursor_brain_api_key",
    "display_name": "Cursor Brain",
    "can_list_models": true
  }
}
```

### 安装、配置与使用体验

- **安装**：先启动本服务（通过 crates.io 安装则运行 `ironclaw-cursor-brain`，从源码则 `cargo run` 或运行 release 二进制），默认监听 `http://127.0.0.1:3001`。
- **配置**：运行 `ironclaw onboard`，在 LLM 步骤中选择 **Cursor Brain**；向导会提示输入 Base URL，可直接回车使用默认 `http://127.0.0.1:3001/v1`，无需 API Key。若服务在其他主机/端口，可填对应地址。
- **使用**：设置 `LLM_BACKEND=cursor`（或 `cursor_brain`）。可选环境变量：`CURSOR_BRAIN_BASE_URL`、`CURSOR_BRAIN_MODEL`（覆盖默认模型）。

完成后，Ironclaw 会像使用其他 OpenAiCompletions provider 一样使用本服务；默认端口 3001 为 Ironclaw 生态约定，可与 Web Gateway（3000）区分。

## 会话延续（可选）

请求时在 HTTP 头中带上配置的 session 头（默认 `X-Session-Id`）并传同一值，本服务会维护「外部 session id → cursor session_id」映射（进程内 LRU，容量可配置）。下次同 session 请求将使用 `--resume` 延续对话。若本次使用了 resume 但返回内容为空，会清除该映射并自动重试一次（不传 resume）。

映射会持久化到 `~/.ironclaw/cursor-brain-sessions.json`（每次写盘采用临时文件 + rename）。重启后保留该文件即可延续会话。

## API

| 端点                        | 说明                                                            |
| --------------------------- | --------------------------------------------------------------- |
| `POST /v1/chat/completions` | OpenAI 格式请求体；支持 `stream: true`（SSE）与 `stream: false` |
| `GET /v1/models`            | 模型列表（见下）                                                |
| `GET /v1/health`            | 健康与 cursor 可用性                                            |

**GET /v1/models 的用途**：返回本服务支持的模型 id 列表。Ironclaw 在配置 LLM 时会请求该接口，并在向导中把列表展示给用户**选择**；用户选中的项会作为 Cursor 的 model。**模型列表由 cursor-agent 命令行查询得到**：每次请求 GET /v1/models 时，插件会执行 `cursor-agent --list-models`（或当前配置的 agent 路径），解析其标准输出并返回；**不由用户配置，也不落盘存储**。若 agent 不可用或查询超时（约 15 秒），则返回默认列表 `["auto", "cursor-default"]`。

请求体中的 `temperature`、`max_tokens` 会解析但不转发给 cursor-agent（使用其默认值）。

## 许可与参考

- **许可：** [LICENSE](LICENSE)（MIT）。
- **参与贡献：** [CONTRIBUTING.md](CONTRIBUTING.md)。
- **架构说明：** [doc/ARCHITECTURE.zh-CN.md](doc/ARCHITECTURE.zh-CN.md)。
- 实现参考：[openclaw-cursor-brain](https://github.com/openclaw/openclaw-cursor-brain)（cursor-agent 封装、stream-json）。
- 与 Ironclaw 的集成仅通过 provider 注册契约，不引入 OpenClaw 概念。
