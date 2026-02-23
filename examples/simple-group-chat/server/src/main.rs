use axum::{Router, routing::get};
use bytes::Bytes;
use postcard::to_allocvec;
use std::net::SocketAddr;
use tower_http::cors::{Any, CorsLayer};
use tracing::{Level, info};

use antenna::server::{AntennaServer, RoomBehavior, RoomContext, signaling::ws_axum_handler, antenna_room, antenna_logic};
use antenna::utils::{Packet, PeerId};
use shared::{ChatClientMsg, ChatServerMsg};
use std::env;

#[antenna_room]
#[derive(Default)]
struct ChatRoom;

#[antenna_logic]
impl ChatRoom {
    async fn on_join(&self, ctx: &RoomContext, user_id: PeerId) {
        info!(">>> User joined the chat: {:?}", user_id);

        let welcome = format!("Welcome to Antenna Chat, {:?}!", user_id);
        ctx.send(&user_id, Bytes::from(welcome)).await;

        let announcement = format!("System: User {:?} has joined.", user_id);
        ctx.broadcast(Bytes::from(announcement)).await;
    }

    #[msg(ChatClientMsg)]
    async fn handle_chat(&self, ctx: &RoomContext, user_id: PeerId, msg: ChatClientMsg) {
        info!("Got msg from {:?}: {:?}", user_id, msg.text);

        let response = ChatServerMsg {
            author_id: user_id.to_string(),
            text: msg.text,
            timestamp: 123456789,
        };

        let response_packet = Packet::User(response);

        if let Ok(response_bytes) = to_allocvec(&response_packet) {
            ctx.broadcast(Bytes::from(response_bytes)).await;
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

    let state = AntennaServer::new()
        .with_ice_server(turn_url, turn_username, turn_credential)
        .build::<ChatRoom>();

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/ws/{user_id}", get(ws_axum_handler))
        .layer(cors)
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    info!("Server listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
