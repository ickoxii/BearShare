use futures_util::{SinkExt, StreamExt};
use std::io::{self, BufRead};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;

use final_project_group6_f25::doc::{Document, Op};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let url = "ws://127.0.0.1:9001";
    let (ws_stream, _) = connect_async(url).await?;
    println!("Connected to {url}");

    let (mut ws_tx, mut ws_rx) = ws_stream.split();

    // Shared local document so all tasks can edit it
    let doc = Arc::new(Mutex::new(Document::new()));

    // Receive the task
    let doc_for_recv = doc.clone();
    let recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = ws_rx.next().await {

            if let Message::Text(s) = &msg {
                println!("[client] got text message: '{s}'");
                if let Some(rest) = s.strip_prefix("SNAPSHOT:") {
                    let mut d = doc_for_recv.lock().await;
                    *d = Document::from_text(rest.to_string());
                    println!("\n[init  ] doc = {}", d.text());
                    print!("> ");
                }

                continue;
            }
            //Only handle binary messages
            let bytes = match msg {
                Message::Binary(b) => b,
                Message::Ping(_) | Message::Pong(_) | Message::Close(_) => continue,
                _ => continue,
            };

            if let Ok(op) = Op::from_bytes(&bytes) {
                let mut d = doc_for_recv.lock().await;

                match op {
                    Op::Insert { pos, text } => {
                        let _ = d.insert(pos, &text);
                    }
                    Op::Delete { pos, len } => {
                        let _ = d.delete(pos, len);
                    }
                }

                println!("\n[remote] doc = {}", d.text());
                print!("> ");
            }
        }
    });

    // Getting the inputs from stdin
    println!("Commands:");
    println!("  i <pos> <text>");
    println!("  d <pos> <len>");

    print!("> ");
    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        let line = line?;
        let mut parts = line.splitn(3, ' ');
        let cmd = parts.next().unwrap_or("");

        if cmd == "i" {
            let pos: usize = parts.next().unwrap_or("0").parse().unwrap_or(0);
            let text = parts.next().unwrap_or("");

            let op = Op::Insert { pos, text: text.to_string() };

            let should_send = {
                let mut d = doc.lock().await;
                match d.insert(pos, text) {
                    Ok(()) => {
                        println!("[local ] doc = {}", d.text());
                        true
                    }
                    Err(e) => {
                        println!("[error ] insert failed: {e}");
                        false
                    }
                }
            }; // lock released here

            ws_tx.send(Message::Binary(op.to_bytes())).await?;
        } else if cmd == "d" {
            let pos: usize = parts.next().unwrap_or("0").parse().unwrap_or(0);
            let len: usize = parts.next().unwrap_or("0").parse().unwrap_or(0);

            let op = Op::Delete { pos, len };

            let should_send = {
                let mut d = doc.lock().await;
                match d.delete(pos, len) {
                    Ok(()) => {
                        println!("[local ] doc = {}", d.text());
                        true
                    }
                    Err(e) => {
                        println!("[error ] delete failed: {e}");
                        false
                    }
                }
            };

            if should_send {
                ws_tx.send(Message::Binary(op.to_bytes())).await?;
            }
        } else {
            println!("Unknown command");
        }

        print!("> ");
    }

    let _ = recv_task.await;
    Ok(())
}
