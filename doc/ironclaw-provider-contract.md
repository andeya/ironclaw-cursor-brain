# Ironclaw OpenAiCompletions 契约核对（第一性原理）

## 1. Ironclaw 如何调用插件

- **配置**：`~/.ironclaw/providers.json` 中 `protocol: "OpenAiCompletions"`、`default_base_url: "http://127.0.0.1:3001/v1"`。
- **解析**：`LlmConfig::resolve()` → `create_openai_compat_from_registry()` → 使用 **rig-core** 的 `openai::Client`，`base_url` 已含 `/v1`，请求发往 `{base_url}/chat/completions`，即 `http://127.0.0.1:3001/v1/chat/completions`。
- **调用路径**：Agent 循环使用 `complete_with_tools()`（见 `reasoning.rs`），即每次请求都会带 **完整 messages**、**tools**、**tool_choice**；rig 将其转为 OpenAI 风格 JSON 发 POST。

## 2. 契约要点（Ironclaw/rig 端）

| 项目         | 契约                                                                                                                                       |
| ------------ | ------------------------------------------------------------------------------------------------------------------------------------------ |
| URL          | `POST {base_url}/chat/completions`，base_url 以 `/v1` 结尾                                                                                 |
| Body         | `model`, `messages`（完整对话，含 system/user/assistant/tool），`stream?`, `temperature?`, `max_tokens?`, **`tools?`**, **`tool_choice?`** |
| 响应（非流） | `choices[].message` 含 `content` 和/或 **`tool_calls`**；`usage`；`finish_reason`                                                          |
| 流式         | SSE，`data: {"choices":[{"delta":{...}}]}`                                                                                                 |

## 3. 插件当前实现 vs 契约

### 3.1 请求体解析（openai.rs / service.rs）

- **ChatCompletionRequest** 只有：`model`, `messages`, `stream`, `temperature`, `max_tokens`。**未声明 `tools`、`tool_choice`**。
- serde 默认忽略未知字段，故 Ironclaw 传 `tools`/`tool_choice` 时请求仍能反序列化，但插件**完全不使用**这两项。

### 3.2 输入语义（已对齐）

- **当前**：当 `messages.len() > 1` 时，使用 `format_messages_as_prompt(&body.messages)` 将**完整 messages** 合成为单条文本（`System: ... --- User: ... --- Assistant: ... --- Tool result: ...`），作为 cursor-agent 的 stdin；单条消息时仍用 `extract_user_message`。
- **cursor-agent 限制**：仅接受单条 prompt（参数或 stdin），不支持 JSON messages 或 tools API；故插件以「全文合成」方式传入完整对话与工具结果，保证上下文一致。

### 3.3 输出语义

- **当前**：插件始终从 cursor-agent 取**纯文本**，构造成 `choices[].message.content`，**不返回 `tool_calls`**。
- Ironclaw 的 `complete_with_tools()` 会解析 `response.tool_calls`；若始终为空，则被当作「无工具调用、仅文本回复」。这与「把 Cursor 当黑盒、只拿最终文本」的用法一致，但与「Ironclaw 侧工具编排」不兼容。

### 3.4 其他

- **URL/方法**：`POST /v1/chat/completions`、`GET /v1/models`、`GET /v1/health` 与 Ironclaw/rig 使用方式一致。
- **API Key**：cursor 配置为 `api_key_required: false` 时，rig 可能送占位 key；插件未校验 Authorization，可接受。
- **模型**：插件 `list_models` 返回 `cursor-default`；`default_model: "auto"` 时请求里会是 `model: "auto"` 等，插件可接受。

## 4. 结论与建议

### 4.1 结论

- **已对齐**：完整 **messages**（含 system/user/assistant/tool）在插件内合成为单条 prompt 文本传给 cursor-agent，与 OpenAiCompletions 的「完整对话」语义一致；单条消息时仍只传该条 user content。
- **未转发**：`tools` / `tool_choice` 仅接收不转发（cursor-agent 无对应 API）；工具结果已体现在 `role: tool` 的 message 中，故会出现在合成 prompt 的「Tool result:」段。

### 4.2 建议

- 若 cursor-agent 未来支持 JSON messages 或 tools，可改为直接透传，减少文本合成带来的格式损失。
- 当前实现已满足「完整对话上下文 + 工具结果以文本形式传入」的契约要求。
