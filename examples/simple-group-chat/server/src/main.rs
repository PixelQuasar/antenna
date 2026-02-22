use async_trait::async_trait;
use axum::{Router, routing::get};
use bytes::Bytes;
use postcard::{from_bytes, to_allocvec};
use std::net::SocketAddr;
use tower_http::cors::{Any, CorsLayer};
use tracing::{Level, info};

use antenna::server::{RoomBehavior, RoomContext, RoomManager, SignalingService, ws_handler};
use antenna::utils::{IceServerConfig, Packet, PeerId};
use shared::{ChatClientMsg, ChatServerMsg};
use std::env;
use std::sync::Arc;

struct ChatRoom;

#[async_trait]
impl RoomBehavior for ChatRoom {
    async fn on_join(&self, ctx: &RoomContext, user_id: PeerId) {
        info!(">>> User joined the chat: {:?}", user_id);

        let welcome = format!("Welcome to Antenna Chat, {:?}!", user_id);
        ctx.send(&user_id, Bytes::from(welcome)).await;

        let announcement = format!("System: User {:?} has joined.", user_id);
        ctx.broadcast(Bytes::from(announcement)).await;
    }

    async fn on_message(&self, ctx: &RoomContext, user_id: PeerId, data: Bytes) {
        println!("{:#?}", ctx);
        println!("{:#?}, {:#?}", user_id, data);
        match from_bytes::<Packet<ChatClientMsg>>(&data) {
            Ok(Packet::User(client_msg)) => {
                println!("Got msg from {:?}: {:?}", user_id, client_msg.text);

                let response = ChatServerMsg {
                    author_id: user_id.to_string(),
                    text: client_msg.text,
                    timestamp: 123456789,
                };

                let response_packet = Packet::User(response);

                if let Ok(response_bytes) = to_allocvec(&response_packet) {
                    ctx.broadcast(Bytes::from(response_bytes)).await;
                }
            }
            Ok(Packet::System(_)) => {}
            Err(e) => {
                eprintln!("Failed to deserialize message: {}", e);
            }
            _ => {}
        }
    }

    async fn on_leave(&self, _ctx: &RoomContext, user_id: PeerId) {
        info!("<<< User left the chat: {:?}", user_id);
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    info!("Initializing Chat Server...");

    let turn_url = env::var("TURN_URL").expect("TURN_URL is not set");
    let turn_username = env::var("TURN_USERNAME").ok();
    let turn_credential = env::var("TURN_CREDENTIAL").ok();

    let ice_servers = vec![IceServerConfig {
        urls: vec![turn_url],
        username: turn_username,
        credential: turn_credential,
    }];

    let signaling = SignalingService::new(ice_servers);
    let signaling_arc = Arc::new(signaling.clone());

    let room_manager = RoomManager::new(
        || Box::new(ChatRoom),
        signaling_arc.clone(),
    );

    let state = Arc::new(antenna::server::AppState {
        signaling: signaling.clone(),
        room_manager,
    });

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/ws/{user_id}", get(ws_handler))
        .layer(cors)
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    info!("Signaling server listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
