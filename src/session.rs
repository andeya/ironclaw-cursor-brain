//! Session storage abstraction: external session id <-> cursor session_id mapping.

use crate::config::home_dir;
use async_trait::async_trait;
use lru::LruCache;
use std::collections::HashMap;
use std::num::NonZeroUsize;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::warn;

/// Session store: get/put/remove by external session id.
/// Implementations can be swapped for tests (mock) or future backends (TTL, persistence).
#[async_trait]
pub trait SessionStore: Send + Sync {
    async fn get(&self, external_id: &str) -> Option<String>;
    async fn put(&self, external_id: String, cursor_id: String);
    async fn remove(&self, external_id: &str);
}

/// In-memory LRU-backed session store (process-local). Used in tests or when injecting a non-persistent store.
#[allow(dead_code)]
pub struct LruSessionStore {
    cache: Arc<RwLock<LruCache<String, String>>>,
}

#[allow(dead_code)]
impl LruSessionStore {
    pub fn new(cap: NonZeroUsize) -> Self {
        Self {
            cache: Arc::new(RwLock::new(LruCache::new(cap))),
        }
    }
}

#[async_trait]
impl SessionStore for LruSessionStore {
    async fn get(&self, external_id: &str) -> Option<String> {
        let mut guard = self.cache.write().await;
        guard.get(external_id).cloned()
    }

    async fn put(&self, external_id: String, cursor_id: String) {
        let mut guard = self.cache.write().await;
        guard.put(external_id, cursor_id);
    }

    async fn remove(&self, external_id: &str) {
        let mut guard = self.cache.write().await;
        guard.pop(external_id);
    }
}

/// Expands a path that may start with `~` or `~\` (Windows) to the user's home directory.
fn expand_tilde(path: &str) -> PathBuf {
    let s = path.trim();
    let rest = if s.starts_with("~/") || s == "~" {
        s.strip_prefix('~').unwrap_or("").trim_start_matches('/')
    } else if s.starts_with("~\\") {
        s.strip_prefix('~').unwrap_or("").trim_start_matches('\\')
    } else {
        return PathBuf::from(path);
    };
    home_dir().join(rest)
}

/// File-backed session store: in-memory LRU + JSON file; load on start, flush on put/remove.
pub struct PersistentSessionStore {
    cache: Arc<RwLock<LruCache<String, String>>>,
    file_path: PathBuf,
}

impl PersistentSessionStore {
    /// Create store: load from file_path if present and valid; otherwise start empty.
    pub fn new(path: String, cap: NonZeroUsize) -> Self {
        let file_path = expand_tilde(&path);
        let mut lru = LruCache::new(cap);

        if file_path.exists() {
            if let Ok(data) = std::fs::read_to_string(&file_path) {
                if let Ok(map) = serde_json::from_str::<HashMap<String, String>>(&data) {
                    for (k, v) in map {
                        lru.put(k, v);
                    }
                } else {
                    warn!(
                        "session file invalid JSON, starting with empty store: {}",
                        file_path.display()
                    );
                }
            } else {
                warn!(
                    "could not read session file, starting with empty store: {}",
                    file_path.display()
                );
            }
        }

        Self {
            cache: Arc::new(RwLock::new(lru)),
            file_path,
        }
    }

    /// Serialize current cache to file: write to temp file then rename.
    fn persist_sync(cache: &LruCache<String, String>, file_path: &std::path::Path) {
        let map: HashMap<String, String> =
            cache.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
        let json = match serde_json::to_string(&map) {
            Ok(j) => j,
            Err(e) => {
                warn!("session serialize error: {}", e);
                return;
            }
        };
        if let Some(parent) = file_path.parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                warn!("session file parent dir create failed: {}", e);
                return;
            }
        }
        let dir = file_path.parent().unwrap_or(std::path::Path::new("."));
        let name = file_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("sessions.json");
        let tmp = dir.join(format!(".{}.tmp", name));
        if let Err(e) = std::fs::write(&tmp, json) {
            warn!("session file write failed: {}", e);
            return;
        }
        if let Err(e) = std::fs::rename(&tmp, file_path) {
            warn!("session file rename failed: {}", e);
            let _ = std::fs::remove_file(&tmp);
        }
    }
}

#[async_trait]
impl SessionStore for PersistentSessionStore {
    async fn get(&self, external_id: &str) -> Option<String> {
        let mut guard = self.cache.write().await;
        guard.get(external_id).cloned()
    }

    async fn put(&self, external_id: String, cursor_id: String) {
        {
            let mut guard = self.cache.write().await;
            guard.put(external_id, cursor_id);
            Self::persist_sync(&guard, &self.file_path);
        }
    }

    async fn remove(&self, external_id: &str) {
        {
            let mut guard = self.cache.write().await;
            guard.pop(external_id);
            Self::persist_sync(&guard, &self.file_path);
        }
    }
}
