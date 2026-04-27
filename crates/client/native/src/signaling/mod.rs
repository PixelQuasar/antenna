use antenna_client_shared::{ClientMsg, ServerMsg};
use antenna_protocol::UserMsgPayload;
use anyhow::{Result, anyhow};
use futures::{SinkExt, StreamExt};
use serde::Serialize;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::protocol::Message;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async};

use crate::Peer;

/// Native WebSocket signaling client
pub struct SignalingClient {
    stream: WebSocketStream<MaybeTlsStream<TcpStream>>,
}

impl SignalingClient {
    pub async fn connect(url: &str) -> Result<Self> {
        let (stream, _resp) = connect_async(url)
            .await
            .map_err(|e| anyhow!("WebSocket connect failed: {e}"))?;
        Ok(Self { stream })
    }

    pub async fn join<Msg: UserMsgPayload + Send + Sync + 'static>(
        mut self,
        room_id: String,
        peer: Arc<Peer<Msg>>,
    ) -> Result<()> {
        self.send(&ClientMsg::Join { room_id: &room_id }).await?;

        let text = self.recv_text().await?;
        match parse(&text)? {
            ServerMsg::RequestOffer => {
                let offer = peer.start().await?;
                self.send(&ClientMsg::Offer {
                    room_id: &room_id,
                    offer: &offer,
                })
                .await?;
            }
            ServerMsg::OfferReceived { offer } => {
                let answer = peer.receive_offer(offer).await?;
                self.send(&ClientMsg::Answer {
                    room_id: &room_id,
                    answer: &answer,
                })
                .await?;
            }
            ServerMsg::Error { message } => return Err(anyhow!("{message}")),
            _ => return Err(anyhow!("Unexpected message after join")),
        }

        tokio::spawn(async move {
            loop {
                let text = match self.recv_text().await {
                    Ok(t) => t,
                    Err(_) => break,
                };
                match parse(&text) {
                    Ok(ServerMsg::RequestOffer) => {
                        let offer = match peer.start().await {
                            Ok(o) => o,
                            Err(_) => break,
                        };
                        if self
                            .send(&ClientMsg::Offer {
                                room_id: &room_id,
                                offer: &offer,
                            })
                            .await
                            .is_err()
                        {
                            break;
                        }
                    }
                    Ok(ServerMsg::AnswerReceived { answer }) => {
                        let _ = peer.receive_answer(answer).await;
                    }
                    _ => {}
                }
            }
        });

        Ok(())
    }

    async fn send(&mut self, msg: &impl Serialize) -> Result<()> {
        let text = serde_json::to_string(msg)?;
        self.stream
            .send(Message::Text(text))
            .await
            .map_err(|e| anyhow!("WebSocket send failed: {e}"))
    }

    async fn recv_text(&mut self) -> Result<String> {
        loop {
            let msg = self
                .stream
                .next()
                .await
                .ok_or_else(|| anyhow!("WebSocket stream ended"))?
                .map_err(|e| anyhow!("WebSocket recv failed: {e}"))?;
            match msg {
                Message::Text(t) => return Ok(t.to_string()),
                Message::Close(_) => return Err(anyhow!("WebSocket closed by peer")),
                _ => continue,
            }
        }
    }
}

fn parse(text: &str) -> Result<ServerMsg<'_>> {
    serde_json::from_str(text).map_err(|e| anyhow!("Failed to parse server message: {e}"))
}
