// Main server implementation with WebSocket handling

use crate::database::Database;
use crate::document::Document;
use crate::file_store::{FileStore, StoredDocument};
use crate::room::{Room, SharedRoom};
use anyhow::{anyhow, Context, Result};
use axum::{
    extract::{
        ws::{Message, WebSocket},
        State, WebSocketUpgrade,
    },
    response::Response,
    routing::get,
    Router,
};
use futures_util::{SinkExt, StreamExt}; // For split() and next()
use protocol::messages::{ClientMessage, ServerMessage};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tower_http::cors::{Any, CorsLayer};
use uuid::Uuid;

/// Server state shared across connections
#[derive(Clone)]
pub struct ServerState {
    /// Active rooms
    rooms: Arc<RwLock<HashMap<String, SharedRoom>>>,

    /// Database
    db: Database,

    /// File store
    file_store: Arc<FileStore>,
}

impl ServerState {
    pub async fn new(db: Database, file_store: FileStore) -> Self {
        ServerState {
            rooms: Arc::new(RwLock::new(HashMap::new())),
            db,
            file_store: Arc::new(file_store),
        }
    }

    /// Get or load a room
    async fn get_room(&self, room_id: &str) -> Result<Option<SharedRoom>> {
        // Check if room is already loaded in memory
        {
            let rooms = self.rooms.read().await;
            if let Some(room) = rooms.get(room_id) {
                return Ok(Some(room.clone()));
            }
        }

        // Try to load from database and file store
        if let Some(_room_record) = self.db.get_room(room_id).await? {
            if self.file_store.document_exists(room_id).await {
                let stored_doc = self.file_store.load_document(room_id).await?;

                // Reconstruct room
                let room = self.load_room_from_storage(room_id, stored_doc).await?;

                // Add to memory
                let mut rooms = self.rooms.write().await;
                rooms.insert(room_id.to_string(), room.clone());

                return Ok(Some(room));
            }
        }

        Ok(None)
    }

    /// Load room from storage
    async fn load_room_from_storage(
        &self,
        room_id: &str,
        stored_doc: StoredDocument,
    ) -> Result<SharedRoom> {
        let room_record = self
            .db
            .get_room(room_id)
            .await?
            .ok_or_else(|| anyhow!("Room not found"))?;

        // Reconstruct document
        let doc_id = Uuid::parse_str(&stored_doc.id)?;
        let mut document =
            Document::new(doc_id, stored_doc.filename.clone(), stored_doc.content, 10);

        // Reapply buffered operations
        for op in stored_doc.buffered_ops {
            document.apply_operation(op);
        }

        // Create room (note: we can't get the original password, so verification will use stored hash)
        let created_at = room_record.created_at_parsed()?;

        let room = Room {
            id: room_id.to_string(),
            name: room_record.name,
            password_hash: room_record.password_hash.clone(),
            document: Arc::new(RwLock::new(document)),
            clients: HashMap::new(),
            next_site_id: 1,
            created_at,
        };

        Ok(Arc::new(RwLock::new(room)))
    }

    /// Create a new room
    async fn create_room(
        &self,
        name: String,
        password: String,
        filename: String,
        initial_content: String,
    ) -> Result<String> {
        let room_id = Uuid::new_v4().to_string();

        // Create room in memory
        let room = Room::new(
            room_id.clone(),
            name.clone(),
            &password,
            filename.clone(),
            initial_content.clone(),
        )?;

        // Save to database
        self.db
            .create_room(&room_id, &name, &room.password_hash, &filename)
            .await?;

        // Save to file store
        let stored_doc = {
            let doc = room.document.read().await;
            StoredDocument {
                id: doc.id.to_string(),
                filename: filename.clone(),
                room_id: room_id.clone(),
                content: initial_content,
                buffered_ops: vec![],
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            }
        }; // doc is dropped here, releasing the borrow
        self.file_store.save_document(&stored_doc).await?;

        // Add to memory
        let room_arc = Arc::new(RwLock::new(room));
        self.rooms.write().await.insert(room_id.clone(), room_arc);

        tracing::info!("Created new room: {}", room_id);
        Ok(room_id)
    }

    /// Persist room state to disk
    async fn persist_room(&self, room_id: &str) -> Result<()> {
        let room = self
            .get_room(room_id)
            .await?
            .ok_or_else(|| anyhow!("Room not found"))?;

        let room_guard = room.read().await;
        let doc = room_guard.document.read().await;

        let stored_doc = StoredDocument {
            id: doc.id.to_string(),
            filename: doc.filename.clone(),
            room_id: room_id.to_string(),
            content: doc.get_base_content().to_string(),
            buffered_ops: doc.get_buffered_ops().to_vec(),
            created_at: room_guard.created_at,
            updated_at: chrono::Utc::now(),
        };

        self.file_store.save_document(&stored_doc).await?;
        self.db.touch_room(room_id).await?;

        Ok(())
    }

    /// Remove room if empty
    async fn cleanup_room(&self, room_id: &str) -> Result<()> {
        let room = match self.get_room(room_id).await? {
            Some(r) => r,
            None => return Ok(()),
        };

        let is_empty = {
            let room_guard = room.read().await;
            room_guard.is_empty()
        };

        if is_empty {
            // Persist final state
            self.persist_room(room_id).await?;

            // Remove from memory
            self.rooms.write().await.remove(room_id);

            tracing::info!("Cleaned up empty room: {}", room_id);
        }

        Ok(())
    }
}

/// Handle WebSocket upgrade
pub async fn websocket_handler(State(state): State<ServerState>, ws: WebSocketUpgrade) -> Response {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

/// Handle individual WebSocket connection
async fn handle_socket(socket: WebSocket, state: ServerState) {
    let client_id = Uuid::new_v4();
    tracing::info!("New WebSocket connection: {}", client_id);

    let (mut sender, mut receiver) = socket.split();
    let (tx, mut rx) = mpsc::unbounded_channel::<ServerMessage>();

    // Spawn task to send messages to client
    let send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            let json = match serde_json::to_string(&msg) {
                Ok(j) => j,
                Err(e) => {
                    tracing::error!("Failed to serialize message: {}", e);
                    continue;
                }
            };

            if sender.send(Message::Text(json.into())).await.is_err() {
                break;
            }
        }
    });

    // Track current room
    let mut current_room: Option<String> = None;

    // Handle incoming messages
    while let Some(Ok(msg)) = receiver.next().await {
        if let Message::Text(text) = msg {
            match serde_json::from_str::<ClientMessage>(&text) {
                Ok(client_msg) => {
                    if let Err(e) =
                        handle_client_message(&state, client_id, &tx, client_msg, &mut current_room)
                            .await
                    {
                        tracing::error!("Error handling message: {}", e);
                        let _ = tx.send(ServerMessage::Error {
                            message: e.to_string(),
                        });
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to parse message: {}", e);
                    let _ = tx.send(ServerMessage::Error {
                        message: format!("Invalid message format: {}", e),
                    });
                }
            }
        }
    }

    // Cleanup on disconnect
    if let Some(room_id) = current_room {
        if let Ok(Some(room)) = state.get_room(&room_id).await {
            let _ = room.write().await.remove_client(client_id).await;
            let _ = state.db.remove_user(&client_id.to_string(), &room_id).await;
            let _ = state.cleanup_room(&room_id).await;
        }
    }

    send_task.abort();
    tracing::info!("WebSocket connection closed: {}", client_id);
}

/// Handle a client message
async fn handle_client_message(
    state: &ServerState,
    client_id: Uuid,
    tx: &mpsc::UnboundedSender<ServerMessage>,
    message: ClientMessage,
    current_room: &mut Option<String>,
) -> Result<()> {
    match message {
        ClientMessage::CreateRoom {
            room_name,
            password,
            filename,
            initial_content,
        } => {
            let room_id = state
                .create_room(room_name, password.clone(), filename, initial_content)
                .await?;

            // Join the room
            let room = state
                .get_room(&room_id)
                .await?
                .ok_or_else(|| anyhow!("Failed to get created room"))?;

            let site_id = room.write().await.add_client(client_id, tx.clone()).await?;
            state
                .db
                .add_user(&client_id.to_string(), &room_id, site_id)
                .await?;

            *current_room = Some(room_id.clone());

            tx.send(ServerMessage::RoomCreated {
                room_id,
                site_id,
                num_sites: 10,
            })?;
        }

        ClientMessage::JoinRoom { room_id, password } => {
            let room = state
                .get_room(&room_id)
                .await?
                .ok_or_else(|| anyhow!("Room not found"))?;

            // Verify password
            if !room.read().await.verify_password(&password) {
                return Err(anyhow!("Invalid password"));
            }

            // Add client to room
            let site_id = room.write().await.add_client(client_id, tx.clone()).await?;
            state
                .db
                .add_user(&client_id.to_string(), &room_id, site_id)
                .await?;

            // Send room info
            let (filename, base_content, buffered_ops) = room.read().await.get_room_info().await;

            *current_room = Some(room_id.clone());

            tx.send(ServerMessage::JoinedRoom {
                room_id,
                site_id,
                num_sites: 10,
                filename,
                document_content: base_content,
                buffered_ops,
            })?;
        }

        ClientMessage::LeaveRoom => {
            if let Some(room_id) = current_room.take() {
                if let Some(room) = state.get_room(&room_id).await? {
                    room.write().await.remove_client(client_id).await?;
                    state
                        .db
                        .remove_user(&client_id.to_string(), &room_id)
                        .await?;
                    state.cleanup_room(&room_id).await?;
                }
            }
        }

        ClientMessage::Operation { op } => {
            tracing::info!("Received operation: {:?}", op);

            if let Some(room_id) = current_room.as_ref() {
                let room = state
                    .get_room(room_id)
                    .await?
                    .ok_or_else(|| anyhow!("Room not found"))?;

                // Get site_id for this client
                let site_id = {
                    let room_guard = room.read().await;
                    room_guard
                        .clients
                        .get(&client_id)
                        .map(|c| c.site_id)
                        .ok_or_else(|| anyhow!("Client not found in room"))?
                };

                // Apply operation to document
                {
                    let room_guard = room.read().await;
                    let mut doc = room_guard.document.write().await;
                    doc.apply_operation(op.clone());

                    // Check if checkpoint needed
                    if doc.needs_checkpoint() {
                        let ops_applied = doc.checkpoint();
                        let content = doc.get_content();

                        // Drop locks before broadcasting
                        drop(doc);
                        drop(room_guard);

                        // Broadcast checkpoint
                        room.read()
                            .await
                            .broadcast_checkpoint(content, ops_applied)
                            .await;

                        // Persist to disk
                        state.persist_room(room_id).await?;
                    }
                }

                // Broadcast operation to other clients
                room.read()
                    .await
                    .broadcast_operation(client_id, site_id, op)
                    .await;
            }
        }

        ClientMessage::RequestSync => {
            if let Some(room_id) = current_room.as_ref() {
                let room = state
                    .get_room(room_id)
                    .await?
                    .ok_or_else(|| anyhow!("Room not found"))?;

                let (_, base_content, buffered_ops) = room.read().await.get_room_info().await;

                tx.send(ServerMessage::SyncResponse {
                    document_content: base_content,
                    buffered_ops,
                })?;
            }
        }

        ClientMessage::Ping => {
            tx.send(ServerMessage::Pong)?;
        }
    }

    Ok(())
}

/// Create and configure the server
pub async fn create_server(state: ServerState, addr: SocketAddr) -> Result<()> {
    // Configure CORS
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Build router
    let app = Router::new()
        .route("/ws", get(websocket_handler))
        .layer(cors)
        .with_state(state);

    tracing::info!("Starting server on {}", addr);

    // Start server (axum 0.7 API)
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .context("Failed to bind to address")?;

    axum::serve(listener, app).await.context("Server error")?;

    Ok(())
}
