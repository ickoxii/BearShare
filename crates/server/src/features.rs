use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fmt,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};
use tokio::{
    sync::{broadcast, RwLock},
    time::{sleep, timeout, Duration},
};

/// A saved version entry for a document.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Version {
    pub id: u64,
    pub doc_id: String,
    pub content: String,
    pub author: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub seq: u64,
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Version {} (seq {}) by {:?} at {}",
            self.id, self.seq, self.author, self.timestamp
        )
    }
}

/// In-memory version timeline store. Replace persistence points with DB calls.
#[derive(Clone, Default)]
pub struct VersionStore {
    // Map doc_id -> Vec<Version> ordered by seq ascending
    inner: Arc<RwLock<HashMap<String, Vec<Version>>>>,
    seq: Arc<AtomicU64>,
}

impl VersionStore {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(HashMap::new())),
            seq: Arc::new(AtomicU64::new(1)),
        }
    }

    /// Save a new version for a doc. Persist to DB/filestore where needed.
    pub async fn save_version(
        &self,
        doc_id: impl Into<String>,
        content: impl Into<String>,
        author: Option<String>,
    ) -> Result<Version> {
        let doc_id = doc_id.into();
        let content = content.into();
        let seq = self.seq.fetch_add(1, Ordering::Relaxed);
        let version = Version {
            id: seq,
            doc_id: doc_id.clone(),
            content,
            author,
            timestamp: Utc::now(),
            seq,
        };

        // TODO: persist to database::Database and file_store::FileStore as needed.
        // e.g. db.insert_version(&version).await?;
        {
            let mut map = self.inner.write().await;
            map.entry(doc_id).or_default().push(version.clone());
        }
        Ok(version)
    }

    /// List past versions for a document (most recent last).
    pub async fn list_versions(&self, doc_id: &str) -> Vec<Version> {
        let map = self.inner.read().await;
        map.get(doc_id).cloned().unwrap_or_default()
    }

    /// Get a specific version by seq/id.
    pub async fn get_version(&self, doc_id: &str, seq: u64) -> Option<Version> {
        let map = self.inner.read().await;
        map.get(doc_id)
            .and_then(|v| v.iter().find(|x| x.seq == seq))
            .cloned()
    }

    /// Restore a version: here we return the content to be applied to the live document.
    /// The caller should apply it and create a new version if desired (or mark restore).
    pub async fn restore_version(&self, doc_id: &str, seq: u64) -> Option<Version> {
        self.get_version(doc_id, seq).await
    }

    /// Very small text diff: lines present in new but not in old, and vice-versa.
    /// Not a full-featured diff; replace with a crate like `similar` for better output.
    pub async fn compare_versions(&self, doc_id: &str, a_seq: u64, b_seq: u64) -> Option<String> {
        let a = self.get_version(doc_id, a_seq).await?;
        let b = self.get_version(doc_id, b_seq).await?;

        let a_lines: Vec<&str> = a.content.lines().collect();
        let b_lines: Vec<&str> = b.content.lines().collect();

        let mut out = String::new();
        out.push_str(&format!("Comparing versions {} -> {}\n", a_seq, b_seq));
        out.push_str("--- old\n+++ new\n");

        // Simple line-by-line comparison (not optimal but deterministic).
        let max = a_lines.len().max(b_lines.len());
        for i in 0..max {
            let la = a_lines.get(i).copied();
            let lb = b_lines.get(i).copied();
            match (la, lb) {
                (Some(x), Some(y)) if x == y => {
                    out.push_str(&format!(" {}\n", x));
                }
                (Some(x), Some(y)) => {
                    out.push_str(&format!("-{}\n+{}\n", x, y));
                }
                (Some(x), None) => out.push_str(&format!("-{}\n", x)),
                (None, Some(y)) => out.push_str(&format!("+{}\n", y)),
                (None, None) => {}
            }
        }
        Some(out)
    }
}

/// States for auto-save visibility in the client UI.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AutoSaveState {
    Saving,
    Saved,
    OfflinePending,
}

/// Auto-saver helper. Sends state updates on a broadcast channel.
/// The `save_with_retry` method takes an arbitrary async save function so it can be used with
/// whichever backend (DB, RPC, etc.) you have in the server.
#[derive(Clone)]
pub struct AutoSaver {
    pub state_tx: broadcast::Sender<AutoSaveState>,
    // configuration
    max_retries: usize,
    base_backoff: Duration,
    ack_timeout: Duration,
}

impl AutoSaver {
    pub fn new() -> Self {
        let (tx, _rx) = broadcast::channel(32);
        Self {
            state_tx: tx,
            max_retries: 5,
            base_backoff: Duration::from_millis(200),
            ack_timeout: Duration::from_secs(5),
        }
    }

    /// Subscribe to state changes (server -> client UI).
    pub fn subscribe_states(&self) -> broadcast::Receiver<AutoSaveState> {
        self.state_tx.subscribe()
    }

    /// Generic save with timeout, ACK-like wait, and retries with exponential backoff.
    /// `save_fn` should attempt to persist/send the version and return Ok(()) when done.
    pub async fn save_with_retry<F, Fut>(
        &self,
        version_content: String,
        author: Option<String>,
        mut save_fn: F,
    ) -> Result<()>
    where
        F: Fn(String, Option<String>) -> Fut,
        Fut: std::future::Future<Output = Result<()>> + Send,
    {
        // announce saving
        let _ = self.state_tx.send(AutoSaveState::Saving);

        let mut attempt = 0usize;
        loop {
            attempt += 1;

            // attempt save with timeout to simulate ACK wait
            let fut = save_fn(version_content.clone(), author.clone());
            match timeout(self.ack_timeout, fut).await {
                Ok(Ok(())) => {
                    // success
                    let _ = self.state_tx.send(AutoSaveState::Saved);
                    return Ok(());
                }
                Ok(Err(e)) => {
                    tracing::warn!("save attempt {} failed: {}", attempt, e);
                }
                Err(_) => {
                    tracing::warn!("save attempt {} timed out waiting for ack", attempt);
                }
            }

            if attempt >= self.max_retries {
                tracing::error!("save failed after {} attempts; marking offline pending", attempt);
                let _ = self.state_tx.send(AutoSaveState::OfflinePending);
                // leave it to the caller to persist locally and schedule background retry.
                return Err(anyhow::anyhow!("save failed after {} attempts", attempt));
            }

            // backoff and retry
            let backoff = self.base_backoff * attempt as u32;
            sleep(backoff).await;
            // continue retrying
        }
    }
}

/// Activity / Audit log event
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ActivityEvent {
    pub seq: u64,
    pub doc_id: Option<String>,
    pub user: Option<String>,
    pub action: String, // "edit", "restore", "autosave", etc.
    pub timestamp: DateTime<Utc>,
    pub details: Option<String>,
}

/// Audit log: ordered events + server-side broadcast for clients.
#[derive(Clone)]
pub struct AuditLog {
    seq: Arc<AtomicU64>,
    inner: Arc<RwLock<Vec<ActivityEvent>>>,
    tx: broadcast::Sender<ActivityEvent>,
}

impl AuditLog {
    pub fn new() -> Self {
        let (tx, _rx) = broadcast::channel(64);
        Self {
            seq: Arc::new(AtomicU64::new(1)),
            inner: Arc::new(RwLock::new(Vec::new())),
            tx,
        }
    }

    /// Log an activity; persist to DB where needed and broadcast to subscribers.
    pub async fn log_event(
        &self,
        doc_id: Option<String>,
        user: Option<String>,
        action: impl Into<String>,
        details: Option<String>,
    ) -> Result<ActivityEvent> {
        let seq = self.seq.fetch_add(1, Ordering::Relaxed);
        let event = ActivityEvent {
            seq,
            doc_id,
            user,
            action: action.into(),
            timestamp: Utc::now(),
            details,
        };

        // TODO: persist event to persistent audit table in database::Database.
        {
            let mut inner = self.inner.write().await;
            inner.push(event.clone());
        }

        // broadcast to subscribers (server-side broadcasting)
        let _ = self.tx.send(event.clone());
        Ok(event)
    }

    /// Subscribe to a live stream of events (server can forward these to connected WebSocket clients).
    pub fn subscribe(&self) -> broadcast::Receiver<ActivityEvent> {
        self.tx.subscribe()
    }

    /// Return ordered events (whole history).
    pub async fn list_events(&self, limit: Option<usize>) -> Vec<ActivityEvent> {
        let inner = self.inner.read().await;
        let mut v = inner.clone();
        if let Some(l) = limit {
            if v.len() > l {
                v = v.into_iter().rev().take(l).collect::<Vec<_>>().into_iter().rev().collect();
            }
        }
        v
    }
}