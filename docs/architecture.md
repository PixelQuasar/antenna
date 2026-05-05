# Crate structure

## antenna

Facade crate that the user adds to their project. Re-exports `antenna-protocol` and one of the platform implementations via a feature flag: `web` (WASM) or `native` (tokio). The optional `signaling-client` feature pulls in the bundled signaling client.

## protocol

SansIO core of the protocol. Contains `MeshNodeFSM` and the entire handshake/relay/reconnect automaton. Knows nothing about WebRTC, tokio, or wasm — it interacts with the world through `Input`/`Output` messages. This makes the core testable without a network and portable across platforms.

## client/shared

Common platform-independent abstractions reused in `client/web` and `client/native`: event and callback types (`Event`, `RtcCallbacks`), ICE-server configuration (`IceServerConfig`), the persistent-storage interface for identity, shared constants.

## client/web

Platform implementation for the browser. Compiled to WebAssembly via `wasm-bindgen`, uses the browser's WebRTC API through `web-sys` bindings. Stores identity in `localStorage`, automatically listens for `beforeunload` for a graceful exit.

## client/native

Platform implementation for native Rust. Built on the tokio runtime, uses the `webrtc-rs` crate as the WebRTC stack. Identity is stored in a file whose path is passed in through `Storage`.

## signaling-server

Reference implementation of the bundled signaling server on axum + tokio. WebSocket endpoint, in-memory rooms, no authentication.

## arbitrary-tests

Property-based tests on `MeshNodeFSM` via [proptest](https://crates.io/crates/proptest). Drives random sequences of `Input` messages and checks protocol invariants (mesh completeness, absence of stuck states, correctness of status transitions).

## integration-tests

End-to-end integration tests with a real WebRTC stack (via `client/native`). Verify scenarios for pair bootstrap, mesh extension, graceful leave, reconnect after force-drop. Run via `cargo nextest` (see `.config/nextest.toml`).

# Tests

Run tests with:

```
cargo nextest run
```
