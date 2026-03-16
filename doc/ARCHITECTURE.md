# Architecture

ironclaw-cursor-brain is a Rust HTTP service that exposes the Cursor Agent as an OpenAI-compatible API for [Ironclaw](https://github.com/nearai/ironclaw).

## Component overview

```mermaid
flowchart TB
    subgraph Ironclaw["Ironclaw"]
        A[Agent / complete_with_tools]
    end
    subgraph Plugin["ironclaw-cursor-brain"]
        B[server: Axum routes]
        C[service: CompletionService]
        D[cursor: subprocess + stream-json]
        E[session: SessionStore]
        F[config: load env + file]
    end
    subgraph External["External"]
        G[cursor-agent process]
    end
    A -->|POST /v1/chat/completions| B
    B --> C
    C --> E
    C --> F
    C --> D
    D -->|stdin / stdout| G
```

## Request flow

```mermaid
sequenceDiagram
    participant Client as Ironclaw / HTTP client
    participant Server as server (Axum)
    participant Service as CompletionService
    participant Session as SessionStore
    participant Cursor as cursor (spawn)
    participant Agent as cursor-agent

    Client->>Server: POST /v1/chat/completions (messages, model?, stream?)
    Server->>Service: complete(input) or complete_stream(input)
    Service->>Session: get(external_session_id) for resume
    Service->>Cursor: spawn_cursor_agent(..., resume?)
    Cursor->>Agent: stdin: synthesized prompt
    Agent-->>Cursor: stdout: stream-json lines
    Cursor-->>Service: content / StreamDelta
    Service->>Session: put(ext_id, cursor_session_id)
    Service-->>Server: (output, model, id) or (id, model, rx)
    Server-->>Client: JSON or SSE
```

## Module roles

| Module      | Role                                                                                                                                  |
| ----------- | ------------------------------------------------------------------------------------------------------------------------------------- |
| **main**    | Load config, bind server, graceful shutdown                                                                                           |
| **server**  | Axum routes: `POST /v1/chat/completions`, `GET /v1/models`, `GET /v1/health`; map `CompletionError` to HTTP                           |
| **service** | Build `CompletionInput` from request; resolve session; spawn via cursor; retry (no-content, fallback model); return output            |
| **cursor**  | Spawn cursor-agent subprocess; write prompt to stdin; parse stream-json from stdout; `run_to_completion` / `run_to_completion_stream` |
| **session** | `SessionStore`: external id ↔ cursor session id; `PersistentSessionStore` = LRU + JSON file under `~/.ironclaw/`                      |
| **config**  | Load from env then optional `~/.ironclaw/cursor-brain.json`; resolve `cursor_path` (PATH or platform paths)                           |
| **openai**  | Request/response types; `format_messages_as_prompt`; `build_completion_response`; SSE chunk helpers                                   |

## Config and integration

- **Config dir**: Same as Ironclaw (`~/.ironclaw/` or `%USERPROFILE%\.ironclaw\` on Windows). See [config](../README.md#configuration).
- **Provider contract**: [ironclaw-provider-contract.md](ironclaw-provider-contract.md). Provider definition: [provider-definition.json](provider-definition.json).

## Data flow summary

1. **Chat request** → Server parses body + headers → Service builds `CompletionInput` (user message, model, stream, optional session id).
2. **Session** → If `X-Session-Id` present, Service looks up cursor session id for resume; after run, stores mapping.
3. **Cursor** → Service calls `spawn_cursor_agent` with prompt (single message or `format_messages_as_prompt`), optional `--resume`, `--model`; reads stream-json (session_id, text, result, thinking); returns content or stream.
4. **Response** → Server maps output to OpenAI-style JSON or SSE.
