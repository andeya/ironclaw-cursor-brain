//! Configuration: cursor_path, port, request_timeout_sec.
//! Load from environment variables, then optional file ~/.ironclaw/cursor-brain.json.

use std::path::PathBuf;

/// Default port (Ironclaw Web Gateway 3000 + 1).
pub const DEFAULT_PORT: u16 = 3001;
/// Default request timeout in seconds.
pub const DEFAULT_REQUEST_TIMEOUT_SEC: u64 = 300;
/// Default session cache capacity (LRU).
pub const DEFAULT_SESSION_CACHE_MAX: u32 = 1000;
/// Default HTTP header name for external session id (case-insensitive).
pub const DEFAULT_SESSION_HEADER_NAME: &str = "x-session-id";

/// Resolve user home directory using the same mechanism as Ironclaw (dirs crate).
/// Ironclaw stores config under ~/.ironclaw/; we use the same base so plugin config and providers.json align.
pub fn home_dir() -> PathBuf {
    dirs::home_dir().unwrap_or_else(|| PathBuf::from("."))
}

/// Default model ids returned by GET /v1/models when cursor-agent --list-models fails or is unavailable.
pub const DEFAULT_MODELS_LIST: &[&str] = &["auto", "cursor-default"];

#[derive(Clone, Debug)]
pub struct Config {
    /// Path to cursor-agent executable; empty means detect from PATH or fixed paths.
    pub cursor_path: Option<String>,
    /// Listen port.
    pub port: u16,
    /// Per-request timeout in seconds.
    pub request_timeout_sec: u64,
    /// Max number of session mappings to keep (LRU eviction).
    pub session_cache_max: u32,
    /// Header name for external session id (e.g. X-Session-Id).
    pub session_header_name: String,
    /// When request omits model, use this if set; else "auto".
    pub default_model: Option<String>,
    /// When primary model returns no content, retry once with this model if set.
    pub fallback_model: Option<String>,
}

/// Default session persistence file path under the Ironclaw config directory (not configurable).
pub fn default_session_file_path() -> String {
    home_dir()
        .join(".ironclaw")
        .join("cursor-brain-sessions.json")
        .to_string_lossy()
        .into_owned()
}

impl Config {
    /// Resolve cursor_path: if unset, detect from PATH or platform-specific paths.
    pub fn resolve_cursor_path(&self) -> Option<String> {
        if let Some(ref p) = self.cursor_path {
            if !p.is_empty() && std::path::Path::new(p).exists() {
                return Some(p.clone());
            }
        }
        detect_cursor_path()
    }
}

/// Load config from environment variables.
/// Optional: merge from ~/.ironclaw/cursor-brain.json if present (env overrides file).
pub fn load_config() -> Config {
    let mut cursor_path: Option<String> =
        std::env::var("CURSOR_PATH").ok().filter(|s| !s.is_empty());
    let mut port = DEFAULT_PORT;
    let mut request_timeout_sec = DEFAULT_REQUEST_TIMEOUT_SEC;
    let mut session_cache_max = DEFAULT_SESSION_CACHE_MAX;
    let mut session_header_name = DEFAULT_SESSION_HEADER_NAME.to_string();
    let mut default_model: Option<String> = None;
    let mut fallback_model: Option<String> = None;

    // Optional file: ~/.ironclaw/cursor-brain.json
    let home = home_dir();
    let path = home.join(".ironclaw").join("cursor-brain.json");
    if path.exists() {
        if let Ok(data) = std::fs::read_to_string(&path) {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&data) {
                if cursor_path.is_none() {
                    cursor_path = v
                        .get("cursor_path")
                        .and_then(|c| c.as_str())
                        .map(String::from);
                }
                if let Some(p) = v.get("port").and_then(|p| p.as_u64()) {
                    port = p.clamp(1, 65535) as u16;
                }
                if let Some(t) = v.get("request_timeout_sec").and_then(|t| t.as_u64()) {
                    request_timeout_sec = t.max(1);
                }
                if let Some(n) = v.get("session_cache_max").and_then(|n| n.as_u64()) {
                    session_cache_max = n.clamp(1, 1_000_000) as u32;
                }
                if let Some(s) = v.get("session_header_name").and_then(|s| s.as_str()) {
                    if !s.is_empty() {
                        session_header_name = s.to_string();
                    }
                }
                if let Some(m) = v.get("default_model").and_then(|m| m.as_str()) {
                    if !m.is_empty() {
                        default_model = Some(m.to_string());
                    }
                }
                if let Some(m) = v.get("fallback_model").and_then(|m| m.as_str()) {
                    if !m.is_empty() {
                        fallback_model = Some(m.to_string());
                    }
                }
            }
        }
    }

    // Environment overrides
    if let Ok(p) = std::env::var("CURSOR_PATH") {
        if !p.is_empty() {
            cursor_path = Some(p);
        }
    }
    if let Ok(p) = std::env::var("PORT").or_else(|_| std::env::var("IRONCLAW_CURSOR_BRAIN_PORT")) {
        if let Ok(n) = p.parse::<u16>() {
            port = n;
        }
    }
    if let Ok(t) = std::env::var("REQUEST_TIMEOUT_SEC") {
        if let Ok(n) = t.parse::<u64>() {
            request_timeout_sec = n.max(1);
        }
    }
    if let Ok(n) = std::env::var("SESSION_CACHE_MAX") {
        if let Ok(v) = n.parse::<u32>() {
            session_cache_max = v.clamp(1, 1_000_000);
        }
    }
    if let Ok(s) = std::env::var("SESSION_HEADER_NAME") {
        if !s.is_empty() {
            session_header_name = s;
        }
    }
    if let Ok(m) = std::env::var("CURSOR_BRAIN_DEFAULT_MODEL") {
        if !m.is_empty() {
            default_model = Some(m);
        }
    }
    if let Ok(m) = std::env::var("CURSOR_BRAIN_FALLBACK_MODEL") {
        if !m.is_empty() {
            fallback_model = Some(m);
        }
    }

    Config {
        cursor_path,
        port,
        request_timeout_sec,
        session_cache_max,
        session_header_name,
        default_model,
        fallback_model,
    }
}

/// Platform-specific candidate paths for cursor-agent (same idea as openclaw getCursorSearchPaths).
fn cursor_search_paths() -> Vec<PathBuf> {
    let home = home_dir();
    #[cfg(windows)]
    {
        let local = std::env::var("LOCALAPPDATA").unwrap_or_else(|_| {
            home.join("AppData")
                .join("Local")
                .to_string_lossy()
                .into_owned()
        });
        let local = PathBuf::from(local);
        vec![
            local
                .join("Programs")
                .join("cursor")
                .join("resources")
                .join("app")
                .join("bin")
                .join("agent.exe"),
            local.join("cursor-agent").join("agent.cmd"),
            home.join(".cursor").join("bin").join("agent.exe"),
            home.join(".cursor").join("bin").join("agent.cmd"),
            home.join(".local").join("bin").join("agent.exe"),
        ]
    }
    #[cfg(not(windows))]
    {
        vec![
            home.join(".local").join("bin").join("agent"),
            PathBuf::from("/usr/local/bin/agent"),
            home.join(".cursor").join("bin").join("agent"),
        ]
    }
}

/// Detect cursor-agent from PATH or fixed paths.
fn detect_cursor_path() -> Option<String> {
    // 1) which / where
    #[cfg(windows)]
    let out = std::process::Command::new("where")
        .args(["agent"])
        .output()
        .ok();
    #[cfg(not(windows))]
    let out = std::process::Command::new("which")
        .arg("agent")
        .output()
        .ok();

    if let Some(ref o) = out {
        if o.status.success() {
            let s = String::from_utf8_lossy(&o.stdout);
            let first = s.lines().next()?.trim();
            if !first.is_empty() && std::path::Path::new(first).exists() {
                return Some(first.to_string());
            }
        }
    }

    // 2) Search paths
    for p in cursor_search_paths() {
        if p.exists() {
            return Some(p.to_string_lossy().into_owned());
        }
    }
    None
}
