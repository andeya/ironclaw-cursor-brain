//! OpenAI request/response conversion: extract user message, build completion response and SSE.

use crate::cursor::CompletionOutput;
use serde::{Deserialize, Serialize};

/// OpenAI-style message (from request body).
#[derive(Debug, Clone, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: Option<serde_json::Value>,
}

/// Request body for POST /v1/chat/completions.
/// temperature/max_tokens accepted for API compatibility; cursor-agent uses its own defaults.
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct ChatCompletionRequest {
    pub model: Option<String>,
    pub messages: Vec<ChatMessage>,
    pub stream: Option<bool>,
    #[serde(default)]
    pub temperature: Option<f32>,
    #[serde(default)]
    pub max_tokens: Option<u32>,
}

/// Extract the last user message as plain text (for cursor-agent stdin).
/// content can be string or array of { type: "text", text: "..." }.
pub fn extract_user_message(messages: &[ChatMessage]) -> String {
    for m in messages.iter().rev() {
        if m.role != "user" {
            continue;
        }
        match &m.content {
            Some(serde_json::Value::String(s)) => return s.clone(),
            Some(serde_json::Value::Array(arr)) => {
                let parts: Vec<String> = arr
                    .iter()
                    .filter_map(|c| {
                        let obj = c.as_object()?;
                        if obj.get("type").and_then(|t| t.as_str()) == Some("text") {
                            obj.get("text").and_then(|t| t.as_str()).map(String::from)
                        } else {
                            None
                        }
                    })
                    .collect();
                return parts.join("\n");
            }
            _ => continue,
        }
    }
    String::new()
}

/// Non-stream response: single JSON object.
#[derive(Debug, Serialize)]
pub struct ChatCompletionResponse {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<Choice>,
    pub usage: Usage,
}

#[derive(Debug, Serialize)]
pub struct Choice {
    pub index: u32,
    pub message: Message,
    pub finish_reason: String,
}

#[derive(Debug, Serialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Serialize)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

/// Build non-stream response from CompletionOutput.
pub fn build_completion_response(
    id: &str,
    model: &str,
    out: &CompletionOutput,
    include_thinking_in_content: bool,
) -> ChatCompletionResponse {
    let content = if include_thinking_in_content && !out.thinking_text.is_empty() {
        let block = out
            .thinking_text
            .trim()
            .lines()
            .map(|l| format!("> {}", l))
            .collect::<Vec<_>>()
            .join("\n");
        format!("> 💭 {}\n\n---\n\n{}", block, out.content)
    } else {
        out.content.clone()
    };
    ChatCompletionResponse {
        id: id.to_string(),
        object: "chat.completion".to_string(),
        created: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
        model: model.to_string(),
        choices: vec![Choice {
            index: 0,
            message: Message {
                role: "assistant".to_string(),
                content,
            },
            finish_reason: out.finish_reason.clone(),
        }],
        usage: Usage {
            prompt_tokens: 0,
            completion_tokens: 0,
            total_tokens: 0,
        },
    }
}

/// SSE chunk for streaming (OpenAI format).
#[derive(Debug, Serialize)]
pub struct StreamChunk {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<StreamChoice>,
}

#[derive(Debug, Serialize)]
pub struct StreamChoice {
    pub index: u32,
    pub delta: StreamDelta,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct StreamDelta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
}

/// Format one SSE data line (content delta).
pub fn sse_chunk(
    id: &str,
    model: &str,
    content: Option<&str>,
    finish_reason: Option<&str>,
) -> String {
    let chunk = StreamChunk {
        id: id.to_string(),
        object: "chat.completion.chunk".to_string(),
        created: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
        model: model.to_string(),
        choices: vec![StreamChoice {
            index: 0,
            delta: StreamDelta {
                content: content.map(String::from),
            },
            finish_reason: finish_reason.map(String::from),
        }],
    };
    let json = serde_json::to_string(&chunk).unwrap_or_default();
    format!("data: {}\n\n", json)
}

/// SSE end marker.
pub fn sse_done() -> &'static str {
    "data: [DONE]\n\n"
}
