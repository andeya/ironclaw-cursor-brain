//! Completion service layer: encapsulates session, spawn, retry, and response building.

use crate::config::Config;
use crate::cursor::{run_to_completion, run_to_completion_stream, spawn_cursor_agent, StreamDelta};
use crate::openai::{extract_user_message, ChatCompletionRequest};
use crate::session::SessionStore;
use axum::http::HeaderMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;

use crate::cursor::CompletionOutput;

/// Structured input for a single completion request (built from HTTP body + headers).
#[derive(Clone)]
pub struct CompletionInput {
    pub user_msg: String,
    pub model: String,
    pub stream: bool,
    pub external_session_id: Option<String>,
}

impl CompletionInput {
    /// Build from OpenAI request body and headers; returns None if no user message (caller maps to InvalidRequest).
    pub fn from_request(
        body: &ChatCompletionRequest,
        headers: &HeaderMap,
        session_header_name: &str,
    ) -> Result<Self, CompletionError> {
        let user_msg = extract_user_message(&body.messages);
        if user_msg.is_empty() {
            return Err(CompletionError::InvalidRequest(
                "no user message in messages",
            ));
        }
        let model = body.model.as_deref().unwrap_or("default").to_string();
        let stream = body.stream.unwrap_or(false);
        let external_session_id = headers
            .get(session_header_name)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        Ok(CompletionInput {
            user_msg,
            model,
            stream,
            external_session_id,
        })
    }
}

/// Business errors for completion; handler maps to HTTP status and body.
#[derive(Debug)]
pub enum CompletionError {
    CursorNotFound,
    InvalidRequest(&'static str),
    NoContent,
    SpawnFailed(std::io::Error),
    JoinFailed(String),
}

/// Completion service: owns config, session store, and runs the completion flow.
pub struct CompletionService {
    config: Arc<Config>,
    session_store: Arc<dyn SessionStore>,
    timeout: Duration,
}

impl CompletionService {
    pub fn new(config: Arc<Config>, session_store: Arc<dyn SessionStore>) -> Self {
        let timeout = Duration::from_secs(config.request_timeout_sec);
        Self {
            config,
            session_store,
            timeout,
        }
    }

    /// Resolve cursor path; returns error if not found.
    pub fn cursor_path(&self) -> Result<String, CompletionError> {
        self.config
            .resolve_cursor_path()
            .ok_or(CompletionError::CursorNotFound)
    }

    /// Run non-stream completion: get resume, spawn, session backfill, empty -> remove and retry once.
    pub async fn complete(
        &self,
        input: CompletionInput,
    ) -> Result<(CompletionOutput, String, String), CompletionError> {
        let cursor_path = self.cursor_path()?;
        let resume_session_id = if let Some(ref ext) = input.external_session_id {
            self.session_store.get(ext).await
        } else {
            None
        };

        let (session_tx, mut session_rx) = mpsc::channel::<(String, String)>(4);
        let store = self.session_store.clone();
        tokio::spawn(async move {
            while let Some((ext, cur)) = session_rx.recv().await {
                store.put(ext, cur).await;
            }
        });

        let id = format!("chatcmpl-{}", uuid::Uuid::new_v4().to_simple());
        let mut resume = resume_session_id;
        let out = loop {
            let resume_this = resume.clone();
            let external_session_this = input.external_session_id.clone();
            let session_tx_this = session_tx.clone();
            let cursor_path_this = cursor_path.clone();
            let user_msg_this = input.user_msg.clone();
            let model_this = input.model.clone();
            let timeout = self.timeout;

            let result = tokio::task::spawn_blocking(move || {
                let mut on_session_id = |cursor_id: &str| {
                    if let Some(ref ext) = external_session_this {
                        let _ = session_tx_this.blocking_send((ext.clone(), cursor_id.to_string()));
                    }
                };
                let mut child = spawn_cursor_agent(
                    &cursor_path_this,
                    &user_msg_this,
                    Some(&model_this),
                    resume_this.as_deref(),
                    None,
                )
                .map_err(CompletionError::SpawnFailed)?;
                run_to_completion(&mut child, timeout, Some(&mut on_session_id))
                    .map_err(CompletionError::SpawnFailed)
            })
            .await
            .map_err(|e| CompletionError::JoinFailed(e.to_string()))?;

            let out = result?;
            let empty = out.content.is_empty() && out.thinking_text.is_empty();
            if empty && input.external_session_id.is_some() && resume.is_some() {
                if let Some(ref ext) = input.external_session_id {
                    self.session_store.remove(ext).await;
                }
                resume = None;
                continue;
            }

            if empty {
                return Err(CompletionError::NoContent);
            }
            break out;
        };

        Ok((out, input.model, id))
    }

    /// Run stream completion; returns (id, model, receiver) so handler can build SSE body.
    pub async fn complete_stream(
        &self,
        input: CompletionInput,
    ) -> Result<(String, String, mpsc::Receiver<StreamDelta>), CompletionError> {
        let cursor_path = self.cursor_path()?;
        let resume_session_id = if let Some(ref ext) = input.external_session_id {
            self.session_store.get(ext).await
        } else {
            None
        };

        let (session_tx, mut session_rx) = mpsc::channel::<(String, String)>(4);
        let store = self.session_store.clone();
        tokio::spawn(async move {
            while let Some((ext, cur)) = session_rx.recv().await {
                store.put(ext, cur).await;
            }
        });

        let (tx, rx) = mpsc::channel::<StreamDelta>(32);
        let id = format!("chatcmpl-{}", uuid::Uuid::new_v4().to_simple());
        let cursor_path_clone = cursor_path.clone();
        let model_owned = input.model.clone();
        let user_msg = input.user_msg.clone();
        let external_session_for_spawn = input.external_session_id.clone();
        let timeout = self.timeout;

        tokio::task::spawn_blocking(move || {
            let mut on_session_id = |cursor_id: &str| {
                if let Some(ref ext) = external_session_for_spawn {
                    let _ = session_tx.blocking_send((ext.clone(), cursor_id.to_string()));
                }
            };
            let mut child = match spawn_cursor_agent(
                &cursor_path_clone,
                &user_msg,
                Some(&model_owned),
                resume_session_id.as_deref(),
                None,
            ) {
                Ok(c) => c,
                Err(e) => {
                    let _ = tx.blocking_send(StreamDelta::Done {
                        finish_reason: format!("spawn_error: {}", e),
                    });
                    return;
                }
            };
            let _ = run_to_completion_stream(
                &mut child,
                timeout,
                |delta| {
                    let _ = tx.blocking_send(delta);
                },
                Some(&mut on_session_id),
            );
        });

        Ok((id, input.model, rx))
    }
}
