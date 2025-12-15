use futures_util::{SinkExt, StreamExt};
use std::collections::HashMap;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use tokio::net::TcpListener;
use tokio::sync::{mpsc, Mutex};
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::Message;
use final_project_group6_f25::doc::{Document, Op};

type Tx = mpsc::UnboundedSender<Message>;
type Peers = Arc<Mutex<HashMap<usize, Tx>>>;

//Metadata per operation for server
#[derive(Debug, Clone)]
struct OpMeta {
    seq: u64,   // global sequence number
    client_id: u64,
    client_seq: u64,
}

#[derive(Debug, Clone)]
struct LoggedOp {
    op: Op,
    meta: OpMeta,
}

#[derive(Debug)]
struct ServerState {
    doc: Document,
    log: Vec<LoggedOp>,
    next_seq: u64,
}

impl ServerState {
    fn new() -> Self {
        ServerState {
            doc: Document::new(),
            log: Vec::new(),
            next_seq: 1,
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:9001").await?;
    println!("WS server on ws://127.0.0.1:9001");

    let peers: Peers = Arc::new(Mutex::new(HashMap::new()));
    let next_id = Arc::new(AtomicUsize::new(1));

    let state = Arc::new(Mutex::new(ServerState::new()));

    while let Ok((stream, _addr)) = listener.accept().await {
        let peers = peers.clone();
        let next_id = next_id.clone();
        let state = state.clone();

        tokio::spawn(async move {
            let ws_stream = accept_async(stream)
                .await
                .expect("handshake failed");

            let (mut ws_tx, mut ws_rx) = ws_stream.split();

            // Channel for sending broadcast messages to this client
            let (client_tx, mut client_rx) = mpsc::unbounded_channel::<Message>();

            // Assign an ID and register this client
            let my_id = next_id.fetch_add(1, Ordering::Relaxed);
            {
                let mut guard = peers.lock().await;
                guard.insert(my_id, client_tx.clone());
                println!("Client {my_id} connected");
            }

            // Forwarding broadcast messages to this socket
            let forward_task = tokio::spawn(async move {
                while let Some(msg) = client_rx.recv().await {
                    if ws_tx.send(msg).await.is_err() {
                        break;
                    }
                }
            });

            {
                let st = state.lock().await;
                let snapshot = st.doc.text().to_string();
                println!("[server] sending snapshot to client {my_id}: '{snapshot}'");
                let _ = client_tx.send(Message::Text(format!("SNAPSHOT:{snapshot}")));
            }


            // Reading from this socket and displaying broadcast messages
            while let Some(msg_result) = ws_rx.next().await {
                match msg_result {
                    Ok(Message::Binary(bytes)) => {
                        println!(
                            "[server] received a binary message from client {my_id}, len={}",
                            bytes.len()
                        );

                        let mut should_send = false;

                        match Op::from_bytes(&bytes) {
                            Ok(op) => {
                                let mut st = state.lock().await;

                                let seq = st.next_seq;
                                st.next_seq += 1;

                                let result = match &op {
                                    Op::Insert { pos, text } => st.doc.insert(*pos, text),
                                    Op::Delete { pos, len } => st.doc.delete(*pos, *len),
                                };

                                match result {
                                    Ok(()) => {
                                        println!(
                                            "[server] seq={} from client={} doc = {}",
                                            seq,
                                            my_id,
                                            st.doc.text()
                                        );

                                        st.log.push(LoggedOp {
                                            op: op.clone(),
                                            meta: OpMeta {
                                                seq,
                                                client_id: my_id as u64,
                                                client_seq: 0,
                                            },
                                        });

                                        should_send = true;
                                    }
                                    Err(e) => {
                                        eprintln!("[server] op error from client {my_id}: {e}");
                                        let _ = client_tx.send(Message::Text(format!("ERROR:{e}")));
                                    }
                                }

                                if should_send {
                                    let guard = peers.lock().await;
                                    for (id, tx) in guard.iter() {
                                        if *id != my_id {
                                            let _ = tx.send(Message::Binary(bytes.clone()));
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                eprintln!(
                                    "[server] failed to decode op from client {my_id}: {e}"
                                );
                                let _ = client_tx.send(Message::Text(format!(
                                    "ERROR:could not decode op: {e}"
                                )));
                            }
                        }
                    }

                    Ok(other) => {
                        if let Message::Close(_) = other {
                            println!("Client {my_id} sent close frame");
                            break;
                        }
                    }
                    Err(e) => {
                        eprintln!("WebSocket error from client {my_id}: {e}");
                        break;
                    }
                }
            }

            forward_task.abort();

            // Disconnecting
            {
                let mut guard = peers.lock().await;
                guard.remove(&my_id);
                println!("Client {my_id} disconnected");
            }
        });
    }

    Ok(())
}
