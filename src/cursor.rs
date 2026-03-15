//! cursor-agent subprocess: spawn, stdin write, stdout line-by-line stream-json parsing.
//! See design 4.1 (openclaw-cursor-brain streaming-proxy).

use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};
use std::time::Duration;

/// Parsed stream-json event from cursor-agent stdout.
/// SessionId/ToolCall fields kept for parsing and future session/tool use.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum CursorEvent {
    SessionId(String),
    Thinking { text: String },
    Text(String),
    Result(String),
    ToolCall { subtype: String, tool: String },
}

/// Result of a non-stream completion: content + finish_reason.
#[derive(Debug, Default)]
pub struct CompletionOutput {
    pub content: String,
    pub thinking_text: String,
    pub finish_reason: String,
}

/// Spawn cursor-agent with -p, --output-format stream-json, --stream-partial-output; optional --model, --resume.
/// On Windows, if cursor_path ends with .cmd/.bat, spawn with shell.
pub fn spawn_cursor_agent(
    cursor_path: &str,
    user_msg: &str,
    model: Option<&str>,
    resume_session_id: Option<&str>,
    workspace_dir: Option<&str>,
) -> std::io::Result<Child> {
    let mut args = vec![
        "-p".into(),
        "--output-format".into(),
        "stream-json".into(),
        "--stream-partial-output".into(),
        "--trust".into(),
        "--approve-mcps".into(),
        "--force".into(),
    ];
    if let Some(m) = model {
        if !m.is_empty() && m != "auto" {
            args.push("--model".into());
            args.push(m.to_string());
        }
    }
    if let Some(r) = resume_session_id {
        if !r.is_empty() {
            args.push("--resume".into());
            args.push(r.to_string());
        }
    }

    #[cfg(windows)]
    let needs_shell = cursor_path.to_lowercase().ends_with(".cmd")
        || cursor_path.to_lowercase().ends_with(".bat");
    #[cfg(not(windows))]
    let _needs_shell = false;

    let mut cmd = Command::new(cursor_path);
    cmd.args(&args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(dir) = workspace_dir {
        if !dir.is_empty() {
            cmd.current_dir(dir);
        }
    }
    #[cfg(windows)]
    if needs_shell {
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW not required; use shell
        let cmd_str = format!("\"{}\" {}", cursor_path, args.join(" "));
        cmd = Command::new("cmd");
        cmd.args(["/C", &cmd_str])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
    }

    let mut child = cmd.spawn()?;
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(user_msg.as_bytes());
        let _ = stdin.write_all(b"\n");
        let _ = stdin.flush();
    }
    Ok(child)
}

/// Parse one line of stream-json; returns None if not valid JSON or not an event we care about.
pub fn parse_stream_json_line(line: &str) -> Option<CursorEvent> {
    let line = line.trim();
    if line.is_empty() {
        return None;
    }
    let v: serde_json::Value = serde_json::from_str(line).ok()?;
    let ty = v.get("type")?.as_str()?;
    match ty {
        "session_id" => Some(CursorEvent::SessionId(
            v.get("session_id")?.as_str()?.to_string(),
        )),
        "thinking" => {
            let text = v
                .get("text")
                .and_then(|t| t.as_str())
                .unwrap_or("")
                .to_string();
            if text.is_empty() && v.get("subtype").and_then(|s| s.as_str()) != Some("completed") {
                None
            } else {
                Some(CursorEvent::Thinking { text })
            }
        }
        "text" => v
            .get("text")
            .and_then(|t| t.as_str())
            .map(|s| CursorEvent::Text(s.to_string())),
        "result" => v
            .get("result")
            .and_then(|r| r.as_str())
            .map(|s| CursorEvent::Result(s.to_string())),
        "tool_call" => {
            let subtype = v
                .get("subtype")
                .and_then(|s| s.as_str())
                .unwrap_or("")
                .to_string();
            let tool_call = v.get("tool_call").and_then(|t| t.as_object())?;
            let tool = tool_call.keys().next().cloned().unwrap_or_default();
            Some(CursorEvent::ToolCall { subtype, tool })
        }
        _ => None,
    }
}

/// Run cursor-agent to completion (non-stream): collect stdout, parse, merge result/text into content.
/// Returns when child exits or timeout. On timeout, kills child.
/// If `on_session_id` is provided, it is called when a session_id event is seen (for session mapping).
pub fn run_to_completion(
    child: &mut Child,
    timeout: Duration,
    mut on_session_id: Option<&mut dyn FnMut(&str)>,
) -> std::io::Result<CompletionOutput> {
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| std::io::Error::other("no stdout"))?;
    let reader = BufReader::new(stdout);
    let mut out = CompletionOutput {
        finish_reason: "stop".into(),
        ..Default::default()
    };
    let start = std::time::Instant::now();

    for line in reader.lines() {
        if start.elapsed() > timeout {
            let _ = child.kill();
            break;
        }
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };
        if let Some(ev) = parse_stream_json_line(&line) {
            match ev {
                CursorEvent::Text(s) => out.content.push_str(&s),
                CursorEvent::Result(s) => out.content = s,
                CursorEvent::Thinking { text } => out.thinking_text.push_str(&text),
                CursorEvent::SessionId(s) => {
                    if let Some(f) = &mut on_session_id {
                        f(&s);
                    }
                }
                CursorEvent::ToolCall { subtype, tool } => {
                    tracing::debug!(subtype = %subtype, tool = %tool, "cursor tool_call");
                }
            }
        }
    }
    Ok(out)
}

/// Stream delta for SSE: content chunk or done.
#[derive(Debug, Clone)]
pub enum StreamDelta {
    Content(String),
    Done { finish_reason: String },
}

/// Run cursor-agent and invoke `on_event` for each content delta (for streaming).
/// Callback is invoked from the same thread (blocking); use from spawn_blocking and send to channel.
/// Always invokes on_event(StreamDelta::Done) on exit (normal, timeout, or stdout close) so the receiver can finish.
/// If `on_session_id` is provided, it is called when a session_id event is seen (for session mapping).
pub fn run_to_completion_stream<F>(
    child: &mut Child,
    timeout: Duration,
    mut on_event: F,
    mut on_session_id: Option<&mut dyn FnMut(&str)>,
) -> std::io::Result<()>
where
    F: FnMut(StreamDelta),
{
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| std::io::Error::other("no stdout"))?;
    let reader = BufReader::new(stdout);
    let start = std::time::Instant::now();

    for line in reader.lines() {
        if start.elapsed() > timeout {
            let _ = child.kill();
            on_event(StreamDelta::Done {
                finish_reason: "timeout".to_string(),
            });
            return Ok(());
        }
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };
        if let Some(ev) = parse_stream_json_line(&line) {
            match ev {
                CursorEvent::Text(s) => on_event(StreamDelta::Content(s)),
                CursorEvent::Result(s) => on_event(StreamDelta::Content(s)),
                CursorEvent::Thinking { text } if !text.is_empty() => on_event(
                    StreamDelta::Content(format!("\n\n> 💭 {}\n\n", text.trim())),
                ),
                CursorEvent::SessionId(s) => {
                    if let Some(f) = &mut on_session_id {
                        f(&s);
                    }
                }
                CursorEvent::ToolCall { subtype, tool } => {
                    tracing::debug!(subtype = %subtype, tool = %tool, "cursor tool_call");
                }
                CursorEvent::Thinking { .. } => {}
            }
        }
    }
    on_event(StreamDelta::Done {
        finish_reason: "stop".to_string(),
    });
    Ok(())
}
