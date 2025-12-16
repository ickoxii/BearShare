// Document management with CRDT and checkpointing

use rga::{RemoteOp, Rga};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

const CHECKPOINT_THRESHOLD: usize = 1;

/// Represents a document being collaboratively edited
#[derive(Debug)]
pub struct Document {
    /// Document ID
    pub id: Uuid,

    /// Filename
    pub filename: String,

    /// The CRDT state (master copy + buffered ops)
    pub rga: Rga<char>,

    /// Buffered operations since last checkpoint
    pub buffered_ops: Vec<RemoteOp<char>>,

    /// Base document content (last checkpoint)
    pub base_content: String,

    /// Number of sites (clients) in the room
    pub num_sites: usize,
}

impl Document {
    /// Create a new document with initial content
    pub fn new(id: Uuid, filename: String, initial_content: String, num_sites: usize) -> Self {
        let mut rga = Rga::new(0, num_sites); // Server is site 0

        // Initialize RGA with content
        for (i, ch) in initial_content.chars().enumerate() {
            rga.insert_local(i, ch);
        }

        Document {
            id,
            filename,
            rga,
            buffered_ops: Vec::new(),
            base_content: initial_content,
            num_sites,
        }
    }

    /// Apply a remote operation and buffer it
    pub fn apply_operation(&mut self, op: RemoteOp<char>) {
        self.rga.apply_remote(op.clone());
        self.buffered_ops.push(op);

        // Note: checkpoint is now handled by the server to ensure persistence
    }

    /// Perform checkpoint: apply all buffered ops to base content
    pub fn checkpoint(&mut self) -> usize {
        if self.buffered_ops.is_empty() {
            return 0;
        }

        let ops_count = self.buffered_ops.len();

        // Update base content to current RGA state
        self.base_content = self.get_content();

        // Clear buffered operations
        self.buffered_ops.clear();

        tracing::info!(
            "Checkpoint completed for document {}: {} operations applied",
            self.filename,
            ops_count
        );

        ops_count
    }

    /// Force a checkpoint regardless of threshold
    pub fn force_checkpoint(&mut self) -> usize {
        self.checkpoint()
    }

    /// Get current document content
    pub fn get_content(&self) -> String {
        self.rga.read().into_iter().collect()
    }

    /// Get base content (last checkpoint)
    pub fn get_base_content(&self) -> &str {
        &self.base_content
    }

    /// Get buffered operations since last checkpoint
    pub fn get_buffered_ops(&self) -> &[RemoteOp<char>] {
        &self.buffered_ops
    }

    /// Get number of buffered operations
    pub fn buffered_ops_count(&self) -> usize {
        self.buffered_ops.len()
    }

    /// Check if checkpoint is needed
    pub fn needs_checkpoint(&self) -> bool {
        self.buffered_ops.len() >= CHECKPOINT_THRESHOLD
    }
}

// SAFETY: Document is always wrapped in Arc<RwLock<Document>>, which ensures
// that only one thread can access the RGA CRDT at a time. The RGA uses
// Rc<RefCell<>> internally which is not Send/Sync, but since we guarantee
// single-threaded access through the RwLock, it is safe to implement Send/Sync.
unsafe impl Send for Document {}
unsafe impl Sync for Document {}

/// Shared document state wrapped in Arc<RwLock<>> for concurrent access
pub type SharedDocument = Arc<RwLock<Document>>;

/// Document metadata for storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentMetadata {
    pub id: String,
    pub filename: String,
    pub room_id: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_document_creation() {
        let doc = Document::new(
            Uuid::new_v4(),
            "test.txt".to_string(),
            "Hello".to_string(),
            2,
        );

        assert_eq!(doc.get_content(), "Hello");
        assert_eq!(doc.buffered_ops_count(), 0);
    }

    #[test]
    fn test_checkpoint_threshold() {
        let mut doc = Document::new(Uuid::new_v4(), "test.txt".to_string(), "".to_string(), 2);

        // Add operations up to threshold
        for i in 0..CHECKPOINT_THRESHOLD {
            let op = doc.rga.insert_local(i, 'a').unwrap();
            doc.buffered_ops.push(op);
        }

        assert_eq!(doc.buffered_ops_count(), CHECKPOINT_THRESHOLD);

        // This should trigger checkpoint
        let op = doc.rga.insert_local(CHECKPOINT_THRESHOLD, 'b').unwrap();
        doc.apply_operation(op);

        // Buffered ops should be cleared after checkpoint
        assert_eq!(doc.buffered_ops_count(), 1); // Only the last op
    }

    #[test]
    fn test_force_checkpoint() {
        let mut doc = Document::new(
            Uuid::new_v4(),
            "test.txt".to_string(),
            "Hello".to_string(),
            2,
        );

        // Add a few operations
        for _ in 0..5 {
            let op = doc.rga.insert_local(doc.rga.read().len(), 'x').unwrap();
            doc.buffered_ops.push(op);
        }

        assert_eq!(doc.buffered_ops_count(), 5);

        let applied = doc.force_checkpoint();
        assert_eq!(applied, 5);
        assert_eq!(doc.buffered_ops_count(), 0);
    }
}
