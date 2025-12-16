// Collaborative Editor Client
// Connects to the server via WebSocket and enables real-time document editing

mod secure_channel;

use anyhow::{Context, Result};
use futures_util::{SinkExt, StreamExt};
use rga::RemoteOp;
use secure_channel::{client_handshake, SecureRead, SecureWrite};
use serde::{Deserialize, Serialize};
use std::io::{self, Write};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tokio_tungstenite::{connect_async, tungstenite::Message};

// ============================================================================
// Message Types (must match server's messages.rs)
// ============================================================================

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

    /// Send a CRDT operation (legacy)
    Operation { op: RemoteOp<char> },

    /// Insert text at a position (client-friendly)
    Insert { position: usize, text: String },

    /// Delete text at a position (client-friendly)
    Delete { position: usize, length: usize },

    /// Request current document state
    RequestSync,

    /// Save a version snapshot
    SaveVersion { author: Option<String> },

    /// List all versions for the current document
    ListVersions,

    /// Restore a specific version
    RestoreVersion { seq: u64 },

    /// Compare two versions
    CompareVersions { a_seq: u64, b_seq: u64 },

    /// Get recent activity/audit log
    GetActivityLog { limit: Option<usize> },

    /// Heartbeat/ping
    Ping,
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
        filename: String,
        document_content: String,
    },

    /// Joined room successfully
    JoinedRoom {
        room_id: String,
        site_id: u32,
        num_sites: usize,
        filename: String,
        document_content: String,
        buffered_ops: Vec<RemoteOp<char>>,
    },

    /// Another user joined the room
    UserJoined { user_id: String, site_id: u32 },

    /// Another user left the room
    UserLeft { user_id: String, site_id: u32 },

    /// Incoming CRDT operation from another client
    Operation { from_site: u32, op: RemoteOp<char> },

    /// Document checkpoint reached
    Checkpoint {
        document_content: String,
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

    /// Version saved successfully
    VersionSaved { version: Version },

    /// List of versions
    VersionList { versions: Vec<Version> },

    /// Version restored (contains content to apply)
    VersionRestored { version: Version },

    /// Version comparison diff
    VersionDiff { diff: String },

    /// Activity log events
    ActivityLog { events: Vec<ActivityEvent> },

    /// New activity event (broadcast)
    ActivityEvent { event: ActivityEvent },
}

/// A saved version entry for a document
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Version {
    pub id: u64,
    pub doc_id: String,
    pub content: String,
    pub author: Option<String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub seq: u64,
}

/// Activity / Audit log event
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ActivityEvent {
    pub seq: u64,
    pub doc_id: Option<String>,
    pub user: Option<String>,
    pub action: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub details: Option<String>,
}

// Client State
/// Client state for collaborative editing
#[derive(Debug, Clone)]
struct ClientState {
    /// Our site ID (assigned by server)
    site_id: Option<u32>,
    /// Number of sites in the room
    num_sites: usize,
    /// Current room ID
    room_id: Option<String>,
    /// Document filename
    filename: Option<String>,
    /// Current document content (synced from server)
    content: String,
}

impl ClientState {
    fn new() -> Self {
        ClientState {
            site_id: None,
            num_sites: 10,
            room_id: None,
            filename: None,
            content: String::new(),
        }
    }

    /// Apply a local insert operation
    fn local_insert(&mut self, pos: usize, text: &str) -> bool {
        if pos > self.content.len() {
            return false;
        }
        self.content.insert_str(pos, text);
        true
    }

    /// Apply a local delete operation
    fn local_delete(&mut self, pos: usize, len: usize) -> bool {
        if pos + len > self.content.len() {
            return false;
        }
        self.content.replace_range(pos..pos + len, "");
        true
    }

    /// Apply a remote operation to update local view
    fn apply_remote_op(&mut self, op: &RemoteOp<char>) {
        match op {
            RemoteOp::Insert { value, .. } => {
                // We can't know the exact position without the full CRDT state
                println!("[remote] Insert: '{}'", value);
            }
            RemoteOp::Delete { .. } => {
                println!("[remote] Delete operation");
            }
            RemoteOp::Update { value, .. } => {
                println!("[remote] Update: '{}'", value);
            }
        }
    }
}

// Main Application
#[tokio::main]
async fn main() -> Result<()> {
    // Get server URL from env or use default
    let server_url =
        std::env::var("SERVER_URL").unwrap_or_else(|_| "ws://127.0.0.1:9001/ws".to_string());

    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║           BearShare - Collaborative Editor Client            ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();
    println!("Connecting to server at {}...", server_url);

    // Connect to WebSocket server
    let (ws_stream, _) = connect_async(&server_url)
        .await
        .context("Failed to connect to server")?;

    println!("✓ Connected to server!");

    let (mut ws_tx, mut ws_rx) = ws_stream.split();

    // Perform secure channel handshake
    println!("  Performing secure handshake...");
    let (secure_write, secure_read) = client_handshake(&mut ws_tx, &mut ws_rx)
        .await
        .context("Secure handshake failed")?;
    println!("✓ Secure channel established!");
    println!();

    // Wrap secure channel in Arc<Mutex> for sharing
    let secure_write = Arc::new(Mutex::new(secure_write));
    let secure_read = Arc::new(Mutex::new(secure_read));

    // Shared state
    let state = Arc::new(Mutex::new(ClientState::new()));

    // Channel for sending messages to WebSocket
    let (msg_tx, mut msg_rx) = mpsc::unbounded_channel::<ClientMessage>();

    // Spawn task to send encrypted messages to server
    let secure_write_clone = secure_write.clone();
    let send_task = tokio::spawn(async move {
        while let Some(msg) = msg_rx.recv().await {
            let json = serde_json::to_string(&msg).expect("Failed to serialize message");
            let mut writer = secure_write_clone.lock().await;
            match writer.encrypt(json.as_bytes()) {
                Ok(encrypted) => {
                    if ws_tx.send(Message::Binary(encrypted.into())).await.is_err() {
                        break;
                    }
                }
                Err(e) => {
                    eprintln!("[error] Encryption failed: {}", e);
                    break;
                }
            }
        }
    });

    // Spawn task to receive and decrypt messages from server
    let state_for_recv = state.clone();
    let secure_read_clone = secure_read.clone();
    let recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = ws_rx.next().await {
            if let Message::Binary(data) = msg {
                let mut reader = secure_read_clone.lock().await;
                match reader.decrypt(&data) {
                    Ok(plaintext) => {
                        match String::from_utf8(plaintext) {
                            Ok(text) => {
                                match serde_json::from_str::<ServerMessage>(&text) {
                                    Ok(server_msg) => {
                                        handle_server_message(&state_for_recv, server_msg).await;
                                    }
                                    Err(e) => {
                                        println!("[error] Failed to parse server message: {}", e);
                                    }
                                }
                            }
                            Err(e) => {
                                println!("[error] Invalid UTF-8 in message: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        println!("[error] Decryption failed: {}", e);
                    }
                }
            }
        }
        println!("\n[info] Disconnected from server");
    });

    // Print help
    print_help();

    // Main input loop
    let stdin = io::stdin();
    let mut input = String::new();

    loop {
        print!("> ");
        io::stdout().flush().ok();

        input.clear();
        if stdin.read_line(&mut input).is_err() {
            break;
        }

        let trimmed = input.trim();
        if trimmed.is_empty() {
            continue;
        }

        let parts: Vec<&str> = trimmed.splitn(2, ' ').collect();
        let cmd = parts[0].to_lowercase();
        let args = parts.get(1).copied().unwrap_or("");

        match cmd.as_str() {
            "help" | "h" | "?" => {
                print_help();
            }

            "create" | "c" => {
                if let Err(e) = handle_create_command(args, &msg_tx).await {
                    println!("[error] {}", e);
                }
            }

            "join" | "j" => {
                if let Err(e) = handle_join_command(args, &msg_tx).await {
                    println!("[error] {}", e);
                }
            }

            "leave" | "l" => {
                msg_tx.send(ClientMessage::LeaveRoom).ok();
                let mut state_guard = state.lock().await;
                state_guard.room_id = None;
                state_guard.content.clear();
                println!("[info] Left the room");
            }

            "insert" | "i" => {
                if let Err(e) = handle_insert_command(args, &state, &msg_tx).await {
                    println!("[error] {}", e);
                }
            }

            "delete" | "d" => {
                if let Err(e) = handle_delete_command(args, &state, &msg_tx).await {
                    println!("[error] {}", e);
                }
            }

            "show" | "s" => {
                let state_guard = state.lock().await;
                if state_guard.room_id.is_some() {
                    println!("─────────────────────────────────────────");
                    if state_guard.content.is_empty() {
                        println!("(empty document)");
                    } else {
                        println!("{}", state_guard.content);
                    }
                    println!("─────────────────────────────────────────");
                } else {
                    println!("[info] Not in a room. Use 'create' or 'join' first.");
                }
            }

            "sync" => {
                msg_tx.send(ClientMessage::RequestSync).ok();
                println!("[info] Sync requested");
            }

            "save" => {
                let author = if args.is_empty() {
                    None
                } else {
                    Some(args.to_string())
                };
                msg_tx.send(ClientMessage::SaveVersion { author }).ok();
                println!("[info] Saving version...");
            }

            "versions" | "v" => {
                msg_tx.send(ClientMessage::ListVersions).ok();
                println!("[info] Fetching versions...");
            }

            "restore" => {
                if args.is_empty() {
                    println!("[error] Usage: restore <version_seq>");
                } else if let Ok(seq) = args.parse::<u64>() {
                    msg_tx.send(ClientMessage::RestoreVersion { seq }).ok();
                    println!("[info] Restoring version {}...", seq);
                } else {
                    println!("[error] Invalid version number");
                }
            }

            "diff" => {
                let parts: Vec<&str> = args.split_whitespace().collect();
                if parts.len() < 2 {
                    println!("[error] Usage: diff <seq1> <seq2>");
                } else if let (Ok(a), Ok(b)) = (parts[0].parse::<u64>(), parts[1].parse::<u64>()) {
                    msg_tx
                        .send(ClientMessage::CompareVersions { a_seq: a, b_seq: b })
                        .ok();
                    println!("[info] Comparing versions {} and {}...", a, b);
                } else {
                    println!("[error] Invalid version numbers");
                }
            }

            "activity" | "log" => {
                let limit = if args.is_empty() {
                    None
                } else {
                    args.parse::<usize>().ok()
                };
                msg_tx.send(ClientMessage::GetActivityLog { limit }).ok();
                println!("[info] Fetching activity log...");
            }

            "ping" => {
                msg_tx.send(ClientMessage::Ping).ok();
                println!("[info] Ping sent");
            }

            "status" => {
                let state_guard = state.lock().await;
                println!("─────────────────────────────────────────");
                println!("Room ID:  {:?}", state_guard.room_id);
                println!("Site ID:  {:?}", state_guard.site_id);
                println!("Filename: {:?}", state_guard.filename);
                println!("Content length: {} chars", state_guard.content.len());
                println!("─────────────────────────────────────────");
            }

            "quit" | "exit" | "q" => {
                println!("[info] Goodbye!");
                break;
            }

            _ => {
                println!(
                    "[error] Unknown command '{}'. Type 'help' for available commands.",
                    cmd
                );
            }
        }
    }

    // Cleanup
    send_task.abort();
    recv_task.abort();

    Ok(())
}

// Command Handlers
fn print_help() {
    println!("┌─────────────────────────────────────────────────────────────┐");
    println!("│                      Available Commands                     │");
    println!("├─────────────────────────────────────────────────────────────┤");
    println!("│  create <name> <password> [content]  - Create a new room    │");
    println!("│  join <room_id> <password>           - Join existing room   │");
    println!("│  leave                               - Leave current room   │");
    println!("│  insert <pos> <text>                 - Insert text at pos   │");
    println!("│  delete <pos> <len>                  - Delete len chars     │");
    println!("│  show                                - Show document        │");
    println!("│  sync                                - Request full sync    │");
    println!("├─────────────────────────────────────────────────────────────┤");
    println!("│  save [author]                       - Save version         │");
    println!("│  versions                            - List saved versions  │");
    println!("│  restore <seq>                       - Restore a version    │");
    println!("│  diff <seq1> <seq2>                  - Compare versions     │");
    println!("│  activity [limit]                    - View activity log    │");
    println!("├─────────────────────────────────────────────────────────────┤");
    println!("│  status                              - Show connection info │");
    println!("│  ping                                - Ping server          │");
    println!("│  help                                - Show this help       │");
    println!("│  quit                                - Exit client          │");
    println!("└─────────────────────────────────────────────────────────────┘");
    println!();
    println!("Shortcuts: c=create, j=join, l=leave, i=insert, d=delete, s=show, q=quit");
    println!();
}

async fn handle_create_command(
    args: &str,
    msg_tx: &mpsc::UnboundedSender<ClientMessage>,
) -> Result<()> {
    let parts: Vec<&str> = args.splitn(3, ' ').collect();

    if parts.len() < 2 {
        anyhow::bail!("Usage: create <room_name> <password> [initial_content]");
    }

    let room_name = parts[0].to_string();
    let password = parts[1].to_string();
    let initial_content = parts.get(2).copied().unwrap_or("").to_string();

    msg_tx.send(ClientMessage::CreateRoom {
        room_name: room_name.clone(),
        password,
        filename: format!("{}.txt", room_name),
        initial_content,
    })?;

    println!("[info] Creating room '{}'...", room_name);
    Ok(())
}

async fn handle_join_command(
    args: &str,
    msg_tx: &mpsc::UnboundedSender<ClientMessage>,
) -> Result<()> {
    let parts: Vec<&str> = args.split_whitespace().collect();

    if parts.len() < 2 {
        anyhow::bail!("Usage: join <room_id> <password>");
    }

    let room_id = parts[0].to_string();
    let password = parts[1].to_string();

    msg_tx.send(ClientMessage::JoinRoom {
        room_id: room_id.clone(),
        password,
    })?;

    println!("[info] Joining room {}...", room_id);
    Ok(())
}

async fn handle_insert_command(
    args: &str,
    state: &Arc<Mutex<ClientState>>,
    msg_tx: &mpsc::UnboundedSender<ClientMessage>,
) -> Result<()> {
    let parts: Vec<&str> = args.splitn(2, ' ').collect();

    if parts.len() < 2 {
        anyhow::bail!("Usage: insert <position> <text>");
    }

    let pos: usize = parts[0].parse().context("Position must be a number")?;
    let text = parts[1].to_string();

    let mut state_guard = state.lock().await;

    if state_guard.room_id.is_none() {
        anyhow::bail!("Not in a room. Use 'create' or 'join' first.");
    }

    // Apply locally for immediate feedback
    if !state_guard.local_insert(pos, &text) {
        anyhow::bail!("Insert position out of bounds");
    }

    // Send position-based insert - server handles CRDT conversion and auto-syncs
    msg_tx.send(ClientMessage::Insert {
        position: pos,
        text: text.clone(),
    })?;

    println!("[local] Inserted '{}' at position {}", text, pos);

    Ok(())
}

async fn handle_delete_command(
    args: &str,
    state: &Arc<Mutex<ClientState>>,
    msg_tx: &mpsc::UnboundedSender<ClientMessage>,
) -> Result<()> {
    let parts: Vec<&str> = args.split_whitespace().collect();

    if parts.len() < 2 {
        anyhow::bail!("Usage: delete <position> <length>");
    }

    let pos: usize = parts[0].parse().context("Position must be a number")?;
    let len: usize = parts[1].parse().context("Length must be a number")?;

    let mut state_guard = state.lock().await;

    if state_guard.room_id.is_none() {
        anyhow::bail!("Not in a room. Use 'create' or 'join' first.");
    }

    if !state_guard.local_delete(pos, len) {
        anyhow::bail!(
            "Delete range out of bounds (document has {} chars)",
            state_guard.content.len() + len
        );
    }

    // Send position-based delete - server handles CRDT conversion and auto-syncs
    msg_tx.send(ClientMessage::Delete {
        position: pos,
        length: len,
    })?;

    println!("[local] Deleted {} chars at position {}", len, pos);

    Ok(())
}

// Server Message Handler
async fn handle_server_message(state: &Arc<Mutex<ClientState>>, msg: ServerMessage) {
    match msg {
        ServerMessage::RoomCreated {
            room_id,
            site_id,
            num_sites,
            filename,
            document_content,
        } => {
            let mut state_guard = state.lock().await;
            state_guard.room_id = Some(room_id.clone());
            state_guard.site_id = Some(site_id);
            state_guard.num_sites = num_sites;
            state_guard.filename = Some(filename.clone());
            state_guard.content = document_content.clone();

            println!();
            println!("╔══════════════════════════════════════════════════════════════╗");
            println!("║                     Room Created Successfully                ║");
            println!("╠══════════════════════════════════════════════════════════════╣");
            println!("║  Room ID:  {:<49} ║", room_id);
            println!("║  Site ID:  {:<49} ║", site_id);
            println!("║  Filename: {:<49} ║", filename);
            println!("╠══════════════════════════════════════════════════════════════╣");
            println!("║  Document Content:                                           ║");
            println!("╟──────────────────────────────────────────────────────────────╢");
            if document_content.is_empty() {
                println!("║  (empty document)                                            ║");
            } else {
                for line in document_content.lines() {
                    println!("║  {:<60} ║", line);
                }
            }
            println!("╚══════════════════════════════════════════════════════════════╝");
            println!();
            print!("> ");
            io::stdout().flush().ok();
        }

        ServerMessage::JoinedRoom {
            room_id,
            site_id,
            num_sites,
            filename,
            document_content,
            buffered_ops: _,
        } => {
            let mut state_guard = state.lock().await;
            state_guard.room_id = Some(room_id.clone());
            state_guard.site_id = Some(site_id);
            state_guard.num_sites = num_sites;
            state_guard.filename = Some(filename.clone());
            state_guard.content = document_content.clone();

            println!();
            println!("╔══════════════════════════════════════════════════════════════╗");
            println!("║                      Joined Room Successfully                ║");
            println!("╠══════════════════════════════════════════════════════════════╣");
            println!("║  Room ID:  {:<49} ║", room_id);
            println!("║  Site ID:  {:<49} ║", site_id);
            println!("║  Filename: {:<49} ║", filename);
            println!("╠══════════════════════════════════════════════════════════════╣");
            println!("║  Document Content:                                           ║");
            println!("╟──────────────────────────────────────────────────────────────╢");
            if document_content.is_empty() {
                println!("║  (empty document)                                            ║");
            } else {
                for line in document_content.lines() {
                    println!("║  {:<60} ║", line);
                }
            }
            println!("╚══════════════════════════════════════════════════════════════╝");
            println!();
            print!("> ");
            io::stdout().flush().ok();
        }

        ServerMessage::UserJoined { user_id, site_id } => {
            let display_id = if user_id.len() >= 8 {
                &user_id[..8]
            } else {
                &user_id
            };
            println!();
            println!("[info] User {} joined (site {})", display_id, site_id);
            print!("> ");
            io::stdout().flush().ok();
        }

        ServerMessage::UserLeft { user_id, site_id } => {
            let display_id = if user_id.len() >= 8 {
                &user_id[..8]
            } else {
                &user_id
            };
            println!();
            println!("[info] User {} left (site {})", display_id, site_id);
            print!("> ");
            io::stdout().flush().ok();
        }

        ServerMessage::Operation { from_site, op } => {
            let mut state_guard = state.lock().await;
            state_guard.apply_remote_op(&op);
            println!();
            println!("[remote] Operation from site {}", from_site);
            println!("[info] Use 'sync' to update document view");
            print!("> ");
            io::stdout().flush().ok();
        }

        ServerMessage::Checkpoint {
            document_content,
            ops_applied,
        } => {
            let mut state_guard = state.lock().await;
            state_guard.content = document_content.clone();
            println!();
            println!("[info] Checkpoint: {} operations applied", ops_applied);
            println!("[info] Document: {}", document_content);
            print!("> ");
            io::stdout().flush().ok();
        }

        ServerMessage::SyncResponse {
            document_content,
            buffered_ops: _,
        } => {
            let mut state_guard = state.lock().await;
            state_guard.content = document_content.clone();
            println!();
            println!("[sync] Document updated from server");
            println!("[sync] Content: {}", document_content);
            print!("> ");
            io::stdout().flush().ok();
        }

        ServerMessage::Error { message } => {
            println!();
            println!("[error] Server error: {}", message);
            print!("> ");
            io::stdout().flush().ok();
        }

        ServerMessage::Pong => {
            println!();
            println!("[info] Pong received from server");
            print!("> ");
            io::stdout().flush().ok();
        }

        ServerMessage::VersionSaved { version } => {
            println!();
            println!("╔══════════════════════════════════════════════════════════════╗");
            println!("║                      Version Saved                           ║");
            println!("╠══════════════════════════════════════════════════════════════╣");
            println!("║  Version:  {:<49} ║", version.seq);
            println!("║  Author:   {:<49} ║", version.author.as_deref().unwrap_or("(anonymous)"));
            println!("║  Time:     {:<49} ║", version.timestamp.format("%Y-%m-%d %H:%M:%S"));
            println!("╚══════════════════════════════════════════════════════════════╝");
            print!("> ");
            io::stdout().flush().ok();
        }

        ServerMessage::VersionList { versions } => {
            println!();
            println!("╔══════════════════════════════════════════════════════════════╗");
            println!("║                      Saved Versions                          ║");
            println!("╠══════════════════════════════════════════════════════════════╣");
            if versions.is_empty() {
                println!("║  (no versions saved yet)                                     ║");
            } else {
                for v in &versions {
                    let author = v.author.as_deref().unwrap_or("anon");
                    println!(
                        "║  #{:<3} | {:<12} | {:<19} | {} chars  ║",
                        v.seq,
                        author,
                        v.timestamp.format("%Y-%m-%d %H:%M"),
                        v.content.len()
                    );
                }
            }
            println!("╚══════════════════════════════════════════════════════════════╝");
            print!("> ");
            io::stdout().flush().ok();
        }

        ServerMessage::VersionRestored { version } => {
            let mut state_guard = state.lock().await;
            state_guard.content = version.content.clone();
            println!();
            println!("╔══════════════════════════════════════════════════════════════╗");
            println!("║                    Version Restored                          ║");
            println!("╠══════════════════════════════════════════════════════════════╣");
            println!("║  Restored to version {:<39} ║", version.seq);
            println!("╠══════════════════════════════════════════════════════════════╣");
            println!("║  Content:                                                    ║");
            println!("╟──────────────────────────────────────────────────────────────╢");
            for line in version.content.lines().take(5) {
                println!("║  {:<60} ║", line);
            }
            if version.content.lines().count() > 5 {
                println!("║  ... ({} more lines)                                         ║", 
                    version.content.lines().count() - 5);
            }
            println!("╚══════════════════════════════════════════════════════════════╝");
            print!("> ");
            io::stdout().flush().ok();
        }

        ServerMessage::VersionDiff { diff } => {
            println!();
            println!("╔══════════════════════════════════════════════════════════════╗");
            println!("║                      Version Diff                            ║");
            println!("╚══════════════════════════════════════════════════════════════╝");
            println!("{}", diff);
            print!("> ");
            io::stdout().flush().ok();
        }

        ServerMessage::ActivityLog { events } => {
            println!();
            println!("╔══════════════════════════════════════════════════════════════╗");
            println!("║                      Activity Log                            ║");
            println!("╠══════════════════════════════════════════════════════════════╣");
            if events.is_empty() {
                println!("║  (no activity yet)                                           ║");
            } else {
                for e in &events {
                    let user = e.user.as_deref().unwrap_or("system");
                    println!(
                        "║  {} | {:<10} | {:<15} ║",
                        e.timestamp.format("%H:%M:%S"),
                        user,
                        e.action
                    );
                    if let Some(ref details) = e.details {
                        println!("║    └─ {:<54} ║", details);
                    }
                }
            }
            println!("╚══════════════════════════════════════════════════════════════╝");
            print!("> ");
            io::stdout().flush().ok();
        }

        ServerMessage::ActivityEvent { event } => {
            println!();
            println!(
                "[activity] {} - {} by {}",
                event.action,
                event.details.as_deref().unwrap_or(""),
                event.user.as_deref().unwrap_or("system")
            );
            print!("> ");
            io::stdout().flush().ok();
        }
    }
}

