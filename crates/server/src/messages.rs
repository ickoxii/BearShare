// WebSocket message types for client-server communication

use rga::RemoteOp;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Messages sent from client to server
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClientMessage {
    /// Create a new room with a document
    CreateRoom {
        room_name: String,
        password: String,
        filename: String,
        initial_content: String,
    },

    /// Join an existing room
    JoinRoom { room_id: String, password: String },

    /// Leave the current room
    LeaveRoom,

    /// Send a CRDT operation
    Operation { op: RemoteOp<char> },

    /// Request current document state
    RequestSync,

    /// Heartbeat/ping
    Ping,

    /// Explicitly save a document version (manual or autosave trigger)
    SaveVersion {
        doc_id: String,
        content: String,
        author: Option<String>,
    },

    /// List saved versions for a document
    ListVersions {
        doc_id: String,
    },

    /// Restore a specific version
    RestoreVersion {
        doc_id: String,
        seq: u64,
    },

    /// Compare two versions
    CompareVersions {
        doc_id: String,
        a_seq: u64,
        b_seq: u64,
    },

    /// Request recent activity events
    ListActivity {
        limit: Option<usize>,
    },

}

/// Messages sent from server to client
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ServerMessage {
    /// Room created successfully
    RoomCreated {
        room_id: String,
        site_id: u32,
        num_sites: usize,
    },

    /// Joined room successfully
    JoinedRoom {
        room_id: String,
        site_id: u32,
        num_sites: usize,
        filename: String,
        /// Master copy of the document (base state)
        document_content: String,
        /// Buffered operations since last checkpoint
        buffered_ops: Vec<RemoteOp<char>>,
    },

    /// Another user joined the room
    UserJoined { user_id: String, site_id: u32 },

    /// Another user left the room
    UserLeft { user_id: String, site_id: u32 },

    /// Incoming CRDT operation from another client
    Operation { from_site: u32, op: RemoteOp<char> },

    /// Document checkpoint reached (server applied buffered ops)
    Checkpoint {
        /// New base document content after applying buffered ops
        document_content: String,
        /// Cleared buffer
        ops_applied: usize,
    },

    /// Synchronization response
    SyncResponse {
        document_content: String,
        buffered_ops: Vec<RemoteOp<char>>,
    },

    /// Error message
    Error { message: String },

    /// Pong response to ping
    Pong,

    /// Acknowledgement (used by autosave / retries)
    Ack {
        message: String,
    },

    /// Versions list response
    VersionsList {
        doc_id: String,
        versions: Vec<String>, // Display-friendly (via Version::Display)
    },

    /// Version comparison output
    VersionDiff {
        diff: String,
    },

    /// Version restore result
    VersionRestored {
        doc_id: String,
        seq: u64,
        content: String,
    },

    /// Activity feed response
    ActivityList {
        events: Vec<String>,
    },

    /// Live activity event (optional push)
    ActivityEvent {
        event: String,
    },

}

/// Internal message for server-side communication between tasks
#[derive(Debug, Clone)]
pub enum InternalMessage {
    /// Client connected
    ClientConnected {
        client_id: Uuid,
        sender: tokio::sync::mpsc::UnboundedSender<ServerMessage>,
    },

    /// Client disconnected
    ClientDisconnected { client_id: Uuid },

    /// Client message received
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
