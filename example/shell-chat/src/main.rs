use std::sync::Arc;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, BufReader};

use antenna::{Event, MessageCallback, NoArgCallback, Peer, PeerCallback, Storage};

#[derive(Serialize, Deserialize, Clone)]
struct Message(String);

#[tokio::main]
async fn main() -> Result<()> {
    let storage_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "./antenna-identity.json".to_string());
    let storage = Storage::new(storage_path);

    let peer: Arc<Peer<Message>> = Arc::new(Peer::new(storage));

    peer.subscribe(Event::UserMessage(MessageCallback::<Message>::from_fn(
        |peer_id, msg| {
            println!("[{peer_id}] {}", msg.0);
            Ok(())
        },
    )))
    .await;
    peer.subscribe(Event::PeerConnected(PeerCallback::from_fn(|peer_id| {
        println!("* peer connected: {peer_id}");
        Ok(())
    })))
    .await;
    peer.subscribe(Event::PeerDisconnected(PeerCallback::from_fn(|peer_id| {
        println!("* peer disconnected: {peer_id}");
        Ok(())
    })))
    .await;
    peer.subscribe(Event::PeerLost(PeerCallback::from_fn(|peer_id| {
        println!("* peer lost: {peer_id}");
        Ok(())
    })))
    .await;
    peer.subscribe(Event::Available(NoArgCallback::from_fn(|| {
        println!("* mesh available");
        Ok(())
    })))
    .await;
    peer.subscribe(Event::Unavailable(NoArgCallback::from_fn(|| {
        println!("* mesh unavailable");
        Ok(())
    })))
    .await;

    println!("antenna chat — id: {}", peer.my_id().await);
    println!("commands: id | start | offer <base64> | answer <base64> | send <text> | peers | quit");

    let mut lines = BufReader::new(tokio::io::stdin()).lines();
    while let Ok(Some(line)) = lines.next_line().await {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let (cmd, rest) = line.split_once(' ').unwrap_or((line, ""));
        match cmd {
            "id" => println!("{}", peer.my_id().await),
            "start" => match peer.start().await {
                Ok(offer) => println!("offer: {offer}"),
                Err(e) => eprintln!("error: {e:#}"),
            },
            "offer" => match peer.receive_offer(rest).await {
                Ok(answer) => println!("answer: {answer}"),
                Err(e) => eprintln!("error: {e:#}"),
            },
            "answer" => {
                if let Err(e) = peer.receive_answer(rest).await {
                    eprintln!("error: {e:#}");
                }
            }
            "send" => peer.broadcast(Message(rest.to_string())),
            "peers" => {
                let peers = peer.connected_peers().await;
                if peers.is_empty() {
                    println!("(none)");
                } else {
                    for p in peers {
                        println!("{p}");
                    }
                }
            }
            "quit" => {
                peer.leave();
                break;
            }
            _ => println!("unknown command: {cmd}"),
        }
    }

    Ok(())
}
