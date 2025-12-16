// Room management for collaborative editing

use crate::document::{Document, SharedDocument};
use crate::messages::ServerMessage;
use anyhow::{anyhow, Result};
use argon2::password_hash::{rand_core::OsRng, SaltString};
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use rga::RemoteOp;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use uuid::Uuid;

/// Represents a client connected to a room
#[derive(Debug, Clone)]
pub struct Client {
    pub id: Uuid,
    pub site_id: u32,
    pub sender: mpsc::UnboundedSender<ServerMessage>,
}

/// A collaborative editing room
#[derive(Debug)]
pub struct Room {
    /// Room ID
    pub id: String,

    /// Room name (user-friendly)
    pub name: String,

    /// Password hash (Argon2)
    pub(crate) password_hash: String,

    /// The document being edited
    pub document: SharedDocument,

    /// Connected clients
    pub(crate) clients: HashMap<Uuid, Client>,

    /// Next site ID to assign
    pub next_site_id: u32,

    /// Created timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl Room {
    /// Create a new room
    pub fn new(
        id: String,
        name: String,
        password: &str,
        filename: String,
        initial_content: String,
    ) -> Result<Self> {
        // Hash password with Argon2
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let password_hash = argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| anyhow!("Failed to hash password: {}", e))?
            .to_string();

        // Create document (site 0 is reserved for server)
        let doc_id = Uuid::new_v4();
        let document = Document::new(doc_id, filename, initial_content, 10); // Start with 10 sites

        Ok(Room {
            id,
            name,
            password_hash,
            document: Arc::new(RwLock::new(document)),
            clients: HashMap::new(),
            next_site_id: 1, // Start from 1 (0 is server)
            created_at: chrono::Utc::now(),
        })
    }

    /// Verify password
    pub fn verify_password(&self, password: &str) -> bool {
        let parsed_hash = match PasswordHash::new(&self.password_hash) {
            Ok(hash) => hash,
            Err(_) => return false,
        };

        Argon2::default()
            .verify_password(password.as_bytes(), &parsed_hash)
            .is_ok()
    }

    /// Add a client to the room
    pub async fn add_client(
        &mut self,
        client_id: Uuid,
        sender: mpsc::UnboundedSender<ServerMessage>,
    ) -> Result<u32> {
        let site_id = self.next_site_id;
        self.next_site_id += 1;

        let client = Client {
            id: client_id,
            site_id,
            sender,
        };

        self.clients.insert(client_id, client);

        // Notify other clients
        self.broadcast_except(
            client_id,
            ServerMessage::UserJoined {
                user_id: client_id.to_string(),
                site_id,
            },
        )
        .await;

        tracing::info!(
            "Client {} joined room {} as site {}",
            client_id,
            self.id,
            site_id
        );

        Ok(site_id)
    }

    /// Remove a client from the room
    pub async fn remove_client(&mut self, client_id: Uuid) -> Result<()> {
        if let Some(client) = self.clients.remove(&client_id) {
            // Notify other clients
            self.broadcast_except(
                client_id,
                ServerMessage::UserLeft {
                    user_id: client_id.to_string(),
                    site_id: client.site_id,
                },
            )
            .await;

            tracing::info!("Client {} left room {}", client_id, self.id);
        }

        Ok(())
    }

    /// Get client count
    pub fn client_count(&self) -> usize {
        self.clients.len()
    }

    /// Check if room is empty
    pub fn is_empty(&self) -> bool {
        self.clients.is_empty()
    }

    /// Broadcast operation to all clients except sender
    pub async fn broadcast_operation(&self, from_client: Uuid, from_site: u32, op: RemoteOp<char>) {
        let message = ServerMessage::Operation { from_site, op };
        self.broadcast_except(from_client, message).await;
    }

    /// Broadcast checkpoint to all clients
    pub async fn broadcast_checkpoint(&self, content: String, ops_applied: usize) {
        let message = ServerMessage::Checkpoint {
            document_content: content,
            ops_applied,
        };
        self.broadcast(message).await;
    }

    /// Broadcast sync response to all clients (for auto-sync after operations)
    pub async fn broadcast_sync(&self) {
        let doc = self.document.read().await;
        let content = doc.get_content();
        let buffered_ops = doc.get_buffered_ops().to_vec();
        drop(doc);

        let message = ServerMessage::SyncResponse {
            document_content: content,
            buffered_ops,
        };
        self.broadcast(message).await;
    }

    /// Broadcast message to all clients
    async fn broadcast(&self, message: ServerMessage) {
        for client in self.clients.values() {
            let _ = client.sender.send(message.clone());
        }
    }

    /// Broadcast message to all clients except one
    async fn broadcast_except(&self, except: Uuid, message: ServerMessage) {
        for (id, client) in &self.clients {
            if *id != except {
                let _ = client.sender.send(message.clone());
            }
        }
    }

    /// Send message to specific client
    pub async fn send_to_client(&self, client_id: Uuid, message: ServerMessage) -> Result<()> {
        if let Some(client) = self.clients.get(&client_id) {
            client
                .sender
                .send(message)
                .map_err(|e| anyhow!("Failed to send message: {}", e))?;
        }
        Ok(())
    }

    /// Get room info for new joiners
    pub async fn get_room_info(&self) -> (String, String, Vec<RemoteOp<char>>) {
        let doc = self.document.read().await;
        (
            doc.filename.clone(),
            doc.get_base_content().to_string(),
            doc.get_buffered_ops().to_vec(),
        )
    }
}

/// Shared room state
pub type SharedRoom = Arc<RwLock<Room>>;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_room_creation() {
        let room = Room::new(
            "room1".to_string(),
            "Test Room".to_string(),
            "password123",
            "test.txt".to_string(),
            "Hello".to_string(),
        )
        .unwrap();

        assert_eq!(room.id, "room1");
        assert_eq!(room.name, "Test Room");
        assert!(room.verify_password("password123"));
        assert!(!room.verify_password("wrong"));
    }

    #[tokio::test]
    async fn test_client_management() {
        let mut room = Room::new(
            "room1".to_string(),
            "Test Room".to_string(),
            "password123",
            "test.txt".to_string(),
            "Hello".to_string(),
        )
        .unwrap();

        let (tx, _rx) = mpsc::unbounded_channel();
        let client_id = Uuid::new_v4();

        let site_id = room.add_client(client_id, tx).await.unwrap();
        assert_eq!(site_id, 1); // First client gets site ID 1
        assert_eq!(room.client_count(), 1);

        room.remove_client(client_id).await.unwrap();
        assert_eq!(room.client_count(), 0);
        assert!(room.is_empty());
    }
}
