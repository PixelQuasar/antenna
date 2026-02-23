use proc_macro::TokenStream;

mod antenna_client;
mod antenna_room;

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
#[proc_macro_attribute]
pub fn antenna_client(args: TokenStream, input: TokenStream) -> TokenStream {
    antenna_client::antenna_client_impl(args.into(), input.into()).into()
}

/// Marks a struct as an Antenna room.
///
/// This attribute is currently a marker and does not modify the struct definition.
/// It is used in conjunction with `#[antenna_logic]` to define the behavior of the room.
///
/// # Example
///
/// ```rust,ignore
/// use antenna::room::antenna_room;
///
/// #[antenna_room]
/// #[derive(Default)]
/// struct ChatRoom {
///     // Room state fields
/// }
/// ```
#[proc_macro_attribute]
pub fn antenna_room(args: TokenStream, input: TokenStream) -> TokenStream {
    antenna_room::antenna_room_impl(args.into(), input.into()).into()
}

/// Implements the business logic for an Antenna room.
///
/// This macro should be applied to an `impl` block of a struct marked with `#[antenna_room]`.
/// It automatically implements the `RoomBehavior` trait, routing incoming messages to
/// specific handler methods based on the `#[msg(...)]` attribute.
///
/// # Arguments
///
/// * `msg(MessageType)` - Attribute placed on methods to specify which message type they handle.
///
/// # Example
///
/// ```rust,ignore
/// use antenna::room::antenna_logic;
/// use antenna::server::{RoomContext, PeerId};
///
/// #[antenna_logic]
/// impl ChatRoom {
///     #[msg(ChatClientMsg)]
///     async fn handle_message(&self, ctx: &RoomContext, peer_id: PeerId, msg: ChatClientMsg) {
///         // Handle the message
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
#[proc_macro_attribute]
pub fn antenna_logic(args: TokenStream, input: TokenStream) -> TokenStream {
    antenna_room::antenna_logic_impl(args.into(), input.into()).into()
}
