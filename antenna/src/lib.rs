pub mod utils {
    pub use antenna_core::*;
}

#[cfg(feature = "server")]
pub mod server {
    /// Marks a struct as an Antenna room.
    ///
    /// This attribute is currently a marker and does not modify the struct definition.
    /// It is used in conjunction with `#[antenna_logic]` to define the behavior of the room.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use antenna::server::antenna_room;
    ///
    /// #[antenna_room]
    /// struct ChatRoom {
    ///     // Room state fields
    /// }
    /// ```
    pub use antenna_codegen::antenna_room;

    /// Implements the business logic for an Antenna room.
    ///
    /// This macro should be applied to an `impl` block of a struct marked with `#[antenna_room]`.
    /// It automatically implements the `RoomBehavior` trait, routing incoming messages to
    /// specific handler methods based on the `#[handle_user_message(...)]` and `#[handle_system_message]` attributes.
    ///
    /// # Arguments
    ///
    /// * `handle_user_message(MessageType)` - Attribute placed on methods to specify which user message type they handle.
    /// * `handle_system_message` - Attribute placed on methods to specify they handle system messages.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use antenna::room::antenna_logic;
    /// use antenna::server::{RoomContext, PeerId};
    /// use antenna::core::model::SystemMessage;
    ///
    /// #[antenna_logic]
    /// impl ChatRoom {
    ///     #[handle_user_message(ChatClientMsg)]
    ///     async fn handle_message(&self, ctx: &RoomContext, peer_id: PeerId, msg: ChatClientMsg) {
    ///         // Handle the user message
    ///     }
    ///
    ///     #[handle_system_message]
    ///     async fn handle_system(&self, ctx: &RoomContext, peer_id: PeerId, msg: SystemMessage) {
    ///         // Handle system messages (e.g. Ping, Pong, PeerJoined, PeerLeft)
    ///     }
    ///
    ///     async fn on_join(&self, ctx: &RoomContext, peer_id: PeerId) {
    ///         // Handle peer join
    ///     }
    ///
    ///     async fn on_leave(&self, ctx: &RoomContext, peer_id: PeerId) {
    ///         // Handle peer leave
    ///     }
    /// }
    /// ```
    ///
    /// The macro generates:
    /// * Implementation of `RoomBehavior` trait.
    /// * `on_message` method that deserializes incoming packets and dispatches them to the appropriate handler.
    /// * `on_join` and `on_leave` methods if they are defined in the `impl` block.
    pub use antenna_codegen::antenna_logic;

    /// The main entry point for the Antenna server.
    ///
    /// `AntennaServer` is responsible for configuring and building the server application state.
    /// It allows setting up ICE servers and other configurations before creating the `AppState`.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use antenna::server::AntennaServer;
    /// use axum::{Router, routing::get};
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let app_state = AntennaServer::new()
    ///         .with_ice_server("stun:stun.l.google.com:19302", None, None)
    ///         .build::<MyRoomBehavior>();
    ///
    ///     let app = Router::new()
    ///         .route("/ws", get(antenna::server::ws_axum_handler))
    ///         .with_state(app_state);
    ///
    ///     let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    ///     axum::serve(listener, app).await.unwrap();
    /// }
    /// ```
    pub use antenna_server::AntennaServer;
    pub use antenna_server::RoomBehavior;
    pub use antenna_server::RoomContext;
    pub mod signaling {
        /// WebSocket handler for Axum.
        ///
        /// This function handles WebSocket upgrades and manages the connection lifecycle
        /// for Antenna clients. It should be used as a handler in an Axum router.
        ///
        /// # Example
        ///
        /// ```rust,ignore
        /// use antenna::server::signaling::ws_axum_handler;
        /// use axum::{Router, routing::get};
        ///
        /// let app = Router::new()
        ///     .route("/ws", get(ws_axum_handler))
        ///     .with_state(app_state);
        /// ```
        pub use antenna_server::ws_axum_handler;
    }
}

#[cfg(feature = "client")]
pub mod client {
    /// Generates the necessary boilerplate for an Antenna client struct.
    ///
    /// This macro should be applied to a struct that will serve as the client-side representation
    /// of an Antenna room connection. It generates methods for handling events and tracks,
    /// and integrates with the underlying `AntennaEngine`.
    ///
    /// # Arguments
    ///
    /// * `ClientMsg` - The type of messages sent by the client.
    /// * `ServerMsg` - The type of messages received from the server.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use antenna::client::antenna_client;
    /// use wasm_bindgen::prelude::*;
    ///
    /// #[antenna_client(ChatClientMsg, ChatServerMsg)]
    /// struct ChatClient {
    ///     engine: AntennaEngine,
    /// }
    /// ```
    ///
    /// The macro generates:
    /// * `on_event` method to register a callback for server messages.
    /// * `on_track` method to register a callback for new media tracks.
    /// * `add_track` method to add a media track to the connection.
    /// * TypeScript definitions for the callback types.
    pub use antenna_codegen::antenna_client;
    pub use antenna_wasm_gen::AntennaEngine;
    pub use antenna_wasm_gen::EngineConfig;
}
