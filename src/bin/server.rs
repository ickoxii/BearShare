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

type Tx = mpsc::UnboundedSender<Message>;
type Peers = Arc<Mutex<HashMap<usize, Tx>>>;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:9001").await?;
    println!("WS server on ws://127.0.0.1:9001");

    let peers: Peers = Arc::new(Mutex::new(HashMap::new()));
    let next_id = Arc::new(AtomicUsize::new(1));

    while let Ok((stream, _addr)) = listener.accept().await {
        let peers = peers.clone();
        let next_id = next_id.clone();

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

            // Reading from this socket and displaying broadcast messages
            while let Some(Ok(msg)) = ws_rx.next().await {
                if msg.is_binary() {
                    let guard = peers.lock().await;
                    for (id, tx) in guard.iter() {
                        if *id != my_id { // So I don't echo to myself and see everything twice
                            let _ = tx.send(msg.clone());
                        }
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
