use crate::Peer;
use antenna_client_shared::{ClientMsg, ServerMsg};
use antenna_protocol::UserMsgPayload;
use anyhow::{Result, anyhow};
use futures::StreamExt;
use futures::channel::{mpsc, oneshot};
use serde::Serialize;
use std::{cell::RefCell, rc::Rc};
use wasm_bindgen::{JsCast, JsValue, closure::Closure};
use wasm_bindgen_futures::spawn_local;
use web_sys::{MessageEvent, WebSocket};

pub struct SignalingClient {
    ws: WebSocket,
    rx: mpsc::UnboundedReceiver<String>,
    _onmessage_cb: Closure<dyn FnMut(MessageEvent)>,
}

impl SignalingClient {
    pub async fn connect(url: &str) -> Result<Self> {
        let ws = WebSocket::new(url).map_err(|e| anyhow!("WebSocket::new failed: {:?}", e))?;

        let (msg_tx, msg_rx) = mpsc::unbounded::<String>();
        let onmessage_cb = {
            let msg_tx = msg_tx.clone();
            let cb = Closure::<dyn FnMut(MessageEvent)>::new(move |e: MessageEvent| {
                if let Some(text) = e.data().as_string() {
                    let _ = msg_tx.unbounded_send(text);
                }
            });
            ws.set_onmessage(Some(cb.as_ref().unchecked_ref()));
            cb
        };

        let (open_tx, open_rx) = oneshot::channel::<Result<()>>();
        let open_tx = Rc::new(RefCell::new(Some(open_tx)));

        let _onopen_cb = {
            let open_tx = open_tx.clone();
            let cb = Closure::<dyn FnMut()>::new(move || {
                if let Some(tx) = open_tx.borrow_mut().take() {
                    let _ = tx.send(Ok(()));
                }
            });
            ws.set_onopen(Some(cb.as_ref().unchecked_ref()));
            cb
        };

        let _onerror_cb = {
            let open_tx = open_tx.clone();
            let cb = Closure::<dyn FnMut(JsValue)>::new(move |_| {
                if let Some(tx) = open_tx.borrow_mut().take() {
                    let _ = tx.send(Err(anyhow!("WebSocket connection failed")));
                }
            });
            ws.set_onerror(Some(cb.as_ref().unchecked_ref()));
            cb
        };

        open_rx
            .await
            .map_err(|_| anyhow!("Open signal dropped"))??;

        ws.set_onopen(None);
        ws.set_onerror(None);

        Ok(Self {
            ws,
            rx: msg_rx,
            _onmessage_cb: onmessage_cb,
        })
    }

    #[allow(clippy::await_holding_refcell_ref)]
    pub async fn join<Msg: UserMsgPayload + 'static>(
        mut self,
        room_id: String,
        peer: Rc<RefCell<Peer<Msg>>>,
    ) -> Result<()> {
        self.send(&ClientMsg::Join { room_id: &room_id })?;

        let text = self.recv_text().await?;
        match parse(&text)? {
            ServerMsg::RequestOffer => {
                let offer = peer.borrow().start().await?;
                self.send(&ClientMsg::Offer {
                    room_id: &room_id,
                    offer: &offer,
                })?;
            }
            ServerMsg::OfferReceived { offer } => {
                let answer = peer.borrow().receive_offer(offer).await?;
                self.send(&ClientMsg::Answer {
                    room_id: &room_id,
                    answer: &answer,
                })?;
            }
            ServerMsg::Error { message } => return Err(anyhow!("{message}")),
            _ => return Err(anyhow!("Unexpected message after join")),
        }

        spawn_local(async move {
            while let Some(text) = self.rx.next().await {
                match parse(&text) {
                    Ok(ServerMsg::RequestOffer) => {
                        let offer = match peer.borrow().start().await {
                            Ok(o) => o,
                            Err(_) => break,
                        };
                        if self
                            .send(&ClientMsg::Offer {
                                room_id: &room_id,
                                offer: &offer,
                            })
                            .is_err()
                        {
                            break;
                        }
                    }
                    Ok(ServerMsg::AnswerReceived { answer }) => {
                        let _ = peer.borrow().receive_answer(answer).await;
                    }
                    _ => {}
                }
            }
        });

        Ok(())
    }

    fn send(&self, msg: &impl Serialize) -> Result<()> {
        let text = serde_json::to_string(msg)?;
        self.ws
            .send_with_str(&text)
            .map_err(|e| anyhow!("WebSocket send failed: {:?}", e))
    }

    async fn recv_text(&mut self) -> Result<String> {
        self.rx
            .next()
            .await
            .ok_or_else(|| anyhow!("WebSocket closed"))
    }
}

fn parse(text: &str) -> Result<ServerMsg<'_>> {
    serde_json::from_str(text).map_err(|e| anyhow!("Failed to parse server message: {e}"))
}

impl Drop for SignalingClient {
    fn drop(&mut self) {
        self.ws.set_onmessage(None);
        self.ws.set_onopen(None);
        self.ws.set_onerror(None);
        let _ = self.ws.close();
    }
}
