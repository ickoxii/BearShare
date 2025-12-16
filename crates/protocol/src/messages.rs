// WebSocket message types for client-server communication

use chrono::{DateTime, Utc};
use rga::RemoteOp;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// A saved version entry for a document
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Version {
    pub id: u64,
    pub doc_id: String,
    pub content: String,
    pub author: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub seq: u64,
}

// Activity / Audit log event
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ActivityEvent {
    pub seq: u64,
    pub doc_id: Option<String>,
    pub user: Option<String>,
    pub action: String,
    pub timestamp: DateTime<Utc>,
    pub details: Option<String>,
}

// Messages sent from client to server
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClientMessage {
    // Create a new room with a document
    CreateRoom {
        room_name: String,
        password: String,
        filename: String,
        initial_content: String,
    },

    // Join an existing room
    JoinRoom { room_id: String, password: String },

    // Leave the current room
    LeaveRoom,

    // Send a CRDT operation (legacy, for inter-server sync)
    Operation { op: RemoteOp<char> },

    // Insert text at a position (client-friendly)
    Insert { position: usize, text: String },

    // Delete text at a position (client-friendly)
    Delete { position: usize, length: usize },

    // Request current document state
    RequestSync,

    // Save a version snapshot
    SaveVersion { author: Option<String> },

    // List all versions for the current document
    ListVersions,

    // Restore a specific version
    RestoreVersion { seq: u64 },

    // Compare two versions
    CompareVersions { a_seq: u64, b_seq: u64 },

    // Get recent activity/audit log
    GetActivityLog { limit: Option<usize> },

    // Heartbeat/ping
    Ping,
}

// Messages sent from server to client
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ServerMessage {
    // Room created successfully
    RoomCreated {
        room_id: String,
        site_id: u32,
        num_sites: usize,
        filename: String,
        // Initial document content
        document_content: String,
    },

    // Joined room successfully
    JoinedRoom {
        room_id: String,
        site_id: u32,
        num_sites: usize,
        filename: String,
        // Master copy of the document (base state)
        document_content: String,
        // Buffered operations since last checkpoint
        buffered_ops: Vec<RemoteOp<char>>,
    },

    // Another user joined the room
    UserJoined { user_id: String, site_id: u32 },

    // Another user left the room
    UserLeft { user_id: String, site_id: u32 },

    // Incoming CRDT operation from another client
    Operation { from_site: u32, op: RemoteOp<char> },

    // Document checkpoint reached (server applied buffered ops)
    Checkpoint {
        // New base document content after applying buffered ops
        document_content: String,
        // Cleared buffer
        ops_applied: usize,
    },

    // Synchronization response
    SyncResponse {
        document_content: String,
        buffered_ops: Vec<RemoteOp<char>>,
    },

    // Error message
    Error { message: String },

    // Pong response to ping
    Pong,

    // Version saved successfully
    VersionSaved { version: Version },

    // List of versions
    VersionList { versions: Vec<Version> },

    // Version restored (contains content to apply)
    VersionRestored { version: Version },

    // Version comparison diff
    VersionDiff { diff: String },

    // Activity log events
    ActivityLog { events: Vec<ActivityEvent> },

    // New activity event (broadcast)
    ActivityEvent { event: ActivityEvent },
}

// Internal message for server-side communication between tasks
#[derive(Debug, Clone)]
pub enum InternalMessage {
    // Client connected
    ClientConnected {
        client_id: Uuid,
        sender: tokio::sync::mpsc::UnboundedSender<ServerMessage>,
    },

    // Client disconnected
    ClientDisconnected { client_id: Uuid },

    // Client message received
    ClientMessage {
        client_id: Uuid,
        message: ClientMessage,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use rga::S4Vector;

    #[test]
    fn test_message_serialization() {
        let msg = ClientMessage::CreateRoom {
            room_name: "Test Room".to_string(),
            password: "secret".to_string(),
            filename: "document.txt".to_string(),
            initial_content: "Hello World".to_string(),
        };

        let json = serde_json::to_string(&msg).unwrap();
        let deserialized: ClientMessage = serde_json::from_str(&json).unwrap();

        match deserialized {
            ClientMessage::CreateRoom { room_name, .. } => {
                assert_eq!(room_name, "Test Room");
            }
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_operation_message() {
        let op = RemoteOp::Insert {
            left_id: Some(S4Vector::new(1, 0, 1, 1)),
            value: 'a',
            s4v: S4Vector::new(1, 0, 2, 2),
            vector_clock: vec![2, 0],
        };

        let msg = ServerMessage::Operation {
            from_site: 0,
            op: op.clone(),
        };

        let json = serde_json::to_string(&msg).unwrap();
        let deserialized: ServerMessage = serde_json::from_str(&json).unwrap();

        match deserialized {
            ServerMessage::Operation { from_site, .. } => {
                assert_eq!(from_site, 0);
            }
            _ => panic!("Wrong message type"),
        }
    }
}
