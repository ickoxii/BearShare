// File storage for documents and operations

use anyhow::{Context, Result};
use rga::RemoteOp;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

// Stored document state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredDocument {
    pub id: String,
    pub filename: String,
    pub room_id: String,
    // Base content (last checkpoint)
    pub content: String,
    // Buffered operations since last checkpoint
    pub buffered_ops: Vec<RemoteOp<char>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

// File store for managing document persistence
pub struct FileStore {
    root_dir: PathBuf,
}

impl FileStore {
    // Create a new file store
    pub async fn new<P: AsRef<Path>>(root_dir: P) -> Result<Self> {
        let root_dir = root_dir.as_ref().to_path_buf();

        // Create root directory if it doesn't exist
        fs::create_dir_all(&root_dir)
            .await
            .context("Failed to create file store directory")?;

        Ok(FileStore { root_dir })
    }

    // Get path for a document
    fn document_path(&self, room_id: &str) -> PathBuf {
        self.root_dir.join(format!("{room_id}.json"))
    }

    // Get path for a document's actual content file
    fn content_path(&self, room_id: &str, filename: &str) -> PathBuf {
        self.root_dir.join(format!("{room_id}_{filename}"))
    }

    // Save document to disk
    pub async fn save_document(&self, doc: &StoredDocument) -> Result<()> {
        let path = self.document_path(&doc.room_id);

        // Serialize document metadata and buffered ops
        let json = serde_json::to_string_pretty(doc).context("Failed to serialize document")?;

        // Write to temporary file first, then rename (atomic operation)
        let temp_path = path.with_extension("tmp");
        let mut file = fs::File::create(&temp_path)
            .await
            .context("Failed to create temp file")?;

        file.write_all(json.as_bytes())
            .await
            .context("Failed to write document")?;

        file.sync_all().await.context("Failed to sync file")?;
        drop(file);

        fs::rename(&temp_path, &path)
            .await
            .context("Failed to rename temp file")?;

        // Also save the actual content separately for easy access
        let content_path = self.content_path(&doc.room_id, &doc.filename);
        fs::write(&content_path, &doc.content)
            .await
            .context("Failed to write content file")?;

        tracing::debug!("Saved document for room {}", doc.room_id);
        Ok(())
    }

    // Load document from disk
    pub async fn load_document(&self, room_id: &str) -> Result<StoredDocument> {
        let path = self.document_path(room_id);

        let mut file = fs::File::open(&path)
            .await
            .context("Failed to open document file")?;

        let mut contents = String::new();
        file.read_to_string(&mut contents)
            .await
            .context("Failed to read document file")?;

        let doc: StoredDocument =
            serde_json::from_str(&contents).context("Failed to deserialize document")?;

        tracing::debug!("Loaded document for room {}", room_id);
        Ok(doc)
    }

    // Check if document exists
    pub async fn document_exists(&self, room_id: &str) -> bool {
        let path = self.document_path(room_id);
        fs::metadata(&path).await.is_ok()
    }

    // Delete document
    pub async fn delete_document(&self, room_id: &str, filename: &str) -> Result<()> {
        let doc_path = self.document_path(room_id);
        let content_path = self.content_path(room_id, filename);

        // Delete both files, ignore errors if they don't exist
        let _ = fs::remove_file(doc_path).await;
        let _ = fs::remove_file(content_path).await;

        tracing::debug!("Deleted document for room {}", room_id);
        Ok(())
    }

    // List all stored documents
    pub async fn list_documents(&self) -> Result<Vec<String>> {
        let mut entries = fs::read_dir(&self.root_dir)
            .await
            .context("Failed to read directory")?;

        let mut room_ids = Vec::new();

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    room_ids.push(stem.to_string());
                }
            }
        }

        Ok(room_ids)
    }

    // Create a backup of a document
    pub async fn backup_document(&self, room_id: &str) -> Result<()> {
        let src = self.document_path(room_id);
        let backup_name = format!("{}.backup.{}", room_id, chrono::Utc::now().timestamp());
        let dst = self.root_dir.join(backup_name);

        fs::copy(&src, &dst)
            .await
            .context("Failed to create backup")?;

        tracing::info!("Created backup for room {}", room_id);
        Ok(())
    }

    // Clean up old backups
    pub async fn cleanup_backups(&self, room_id: &str, keep: usize) -> Result<()> {
        let pattern = format!("{room_id}.backup.");
        let mut backups = Vec::new();

        let mut entries = fs::read_dir(&self.root_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let filename = entry.file_name();
            let filename_str = filename.to_string_lossy();

            if filename_str.starts_with(&pattern) {
                if let Ok(metadata) = entry.metadata().await {
                    if let Ok(modified) = metadata.modified() {
                        backups.push((entry.path(), modified));
                    }
                }
            }
        }

        // Sort by modification time (newest first)
        backups.sort_by(|a, b| b.1.cmp(&a.1));

        // Delete old backups
        for (path, _) in backups.into_iter().skip(keep) {
            let _ = fs::remove_file(path).await;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_save_and_load() {
        let temp_dir = TempDir::new().unwrap();
        let store = FileStore::new(temp_dir.path()).await.unwrap();

        let doc = StoredDocument {
            id: uuid::Uuid::new_v4().to_string(),
            filename: "test.txt".to_string(),
            room_id: "room1".to_string(),
            content: "Hello World".to_string(),
            buffered_ops: vec![],
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        // Save
        store.save_document(&doc).await.unwrap();

        // Verify exists
        assert!(store.document_exists("room1").await);

        // Load
        let loaded = store.load_document("room1").await.unwrap();
        assert_eq!(loaded.filename, "test.txt");
        assert_eq!(loaded.content, "Hello World");
    }

    #[tokio::test]
    async fn test_backup() {
        let temp_dir = TempDir::new().unwrap();
        let store = FileStore::new(temp_dir.path()).await.unwrap();

        let doc = StoredDocument {
            id: uuid::Uuid::new_v4().to_string(),
            filename: "test.txt".to_string(),
            room_id: "room1".to_string(),
            content: "Hello World".to_string(),
            buffered_ops: vec![],
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        store.save_document(&doc).await.unwrap();
        store.backup_document("room1").await.unwrap();

        // Should have original + backup
        let docs = store.list_documents().await.unwrap();
        assert!(docs.iter().any(|d| d == "room1"));
    }
}
