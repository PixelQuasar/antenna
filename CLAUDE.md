# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

Behavioral guidelines to reduce common LLM coding mistakes. Merge with project-specific instructions as needed.

**Tradeoff:** These guidelines bias toward caution over speed. For trivial tasks, use judgment.

## 1. Think Before Coding

**Don't assume. Don't hide confusion. Surface tradeoffs.**

Before implementing:
- State your assumptions explicitly. If uncertain, ask.
- If multiple interpretations exist, present them - don't pick silently.
- If a simpler approach exists, say so. Push back when warranted.
- If something is unclear, stop. Name what's confusing. Ask.

## 2. Simplicity First

**Minimum code that solves the problem. Nothing speculative.**

- No features beyond what was asked.
- No abstractions for single-use code.
- No "flexibility" or "configurability" that wasn't requested.
- No error handling for impossible scenarios.
- If you write 200 lines and it could be 50, rewrite it.

Ask yourself: "Would a senior engineer say this is overcomplicated?" If yes, simplify.

## 3. Surgical Changes

**Touch only what you must. Clean up only your own mess.**

When editing existing code:
- Don't "improve" adjacent code, comments, or formatting.
- Don't refactor things that aren't broken.
- Match existing style, even if you'd do it differently.
- If you notice unrelated dead code, mention it - don't delete it.

When your changes create orphans:
- Remove imports/variables/functions that YOUR changes made unused.
- Don't remove pre-existing dead code unless asked.

The test: Every changed line should trace directly to the user's request.

## 4. Goal-Driven Execution

**Define success criteria. Loop until verified.**

Transform tasks into verifiable goals:
- "Add validation" → "Write tests for invalid inputs, then make them pass"
- "Fix the bug" → "Write a test that reproduces it, then make it pass"
- "Refactor X" → "Ensure tests pass before and after"

For multi-step tasks, state a brief plan:
```
1. [Step] → verify: [check]
2. [Step] → verify: [check]
3. [Step] → verify: [check]
```

Strong success criteria let you loop independently. Weak criteria ("make it work") require constant clarification.

---

**These guidelines are working if:** fewer unnecessary changes in diffs, fewer rewrites due to overcomplication, and clarifying questions come before implementation rather than after mistakes.

## Project Overview

**Antenna** is a Rust WebRTC P2P mesh networking SDK targeting WebAssembly (browser clients). It enables browser peers to form a mesh network with direct connections, using an external signaling channel for connection negotiation.

- Rust Edition 2024, minimum version 1.89.0
- Cargo workspace with 5 crates + 1 example
- Primary target: `wasm32-unknown-unknown`

## Commands

```bash
# Build
cargo build
cargo build --target wasm32-unknown-unknown

# Test
cargo test

# Lint
cargo clippy
cargo fmt
```

To run a single test:
```bash
cargo test -p <crate-name> <test_name>
# e.g.: cargo test -p antenna-protocol bootstrap_host
```

## Workspace Layout

```
crates/
  protocol/          # antenna-protocol — transport-agnostic FSM core
  antenna/           # antenna — public SDK facade
  client/
    shared/          # antenna-client-shared — Client trait definition
    web/             # antenna-client-web — WASM/WebRTC implementation
    native/          # antenna-client-native — stub, not implemented
  signaling-server/  # antenna-signaling-server — stub, not implemented
example/
  minimal-chat/      # wasm-bindgen example app
docs/
  todo.md            # development roadmap (in Russian)
```

## Architecture

The design follows a **SansIO / layered FSM pattern** — protocol logic is fully transport-agnostic and testable without browser APIs.

### Layer 1 — Protocol (`crates/protocol/`)

Two state machines:

**`MeshNodeFSM`** (`src/mesh/`) — manages the full peer mesh.
- Accepts `Input<Msg>` events, returns `Vec<Output<Msg>>`
- Tracks connected peers (`HashSet<PeerID>`) and in-progress handshakes (`HashMap<PeerID, HandshakeContext>`)
- Inputs: `InitHandshake`, `Handshake`, `MessageReceived`, `Send`, `Broadcast`, `PeerLeaving`
- Outputs: `Handshake`, `SendMessage`, `ReceiveMessage`, `PeerConnected`, `PeerDisconnected`

**`HandshakeFSM`** (`src/handshake/`) — manages individual peer connection negotiation.
- Two roles: **Host** (offer initiator) and **Joiner** (answer responder)
- Two modes: **Bootstrap** (direct new-peer connection) and **Relay** (introduction through existing mesh member)
- State progression: `Idle → CreatingOffer/CreatingAnswer → WaitingForAnswer/WaitingForDataChannel → Connected`

### Layer 2 — WebRTC (`crates/client/web/src/webrtc/`)

Thin wrappers over `web_sys` browser APIs:
- `PeerConnectionManager` — wraps `RtcPeerConnection`: SDP offer/answer, ICE, descriptions
- `DataChannelManager` — wraps `RtcDataChannel`: bidirectional serialized message transport

### Layer 3 — Driver (`crates/client/web/src/driver/`)

**`Driver<Msg>`** bridges the protocol FSM and WebRTC layer:
- Feeds inputs into `MeshNodeFSM`, receives outputs
- Translates protocol outputs into WebRTC operations (create offer/answer, set descriptions, send data)
- Manages per-peer `PeerConnectionManager` and `DataChannelManager` lifecycles
- Routes incoming WebRTC callbacks back to the FSM as inputs

### Layer 4 — Client API (`crates/client/web/src/client/`)

**`Client<Msg>`** — the public high-level async API:
```rust
client.start_bootstrap(peer_id)            // initiate as Host
client.receive_bootstrap_offer(id, offer)  // accept as Joiner
client.receive_answer(peer_id, answer)     // complete handshake
client.send(peer_id, data)
client.broadcast(data)
client.subscribe(callback)
```

Uses `Rc<RefCell<T>>` throughout for interior mutability (required for WASM callback ownership model).

### Message Types (`crates/protocol/src/state/message.rs`)

- `UserMsgPayload` trait: `Serialize + DeserializeOwned + Clone + 'static` — user-defined message type
- `MsgPayload<Msg>` enum: wraps either a user message or an internal `RelayPayload` (mesh signaling)
- `PeerID` is the peer identity type used throughout

### Handshake Signal Flow

The SDK is signaling-channel-agnostic — the caller is responsible for transmitting SDP offers/answers between peers (e.g. via WebSocket, copy-paste, etc.). The SDK surfaces SDP strings through the callback system (`RtcCallbacks`) and accepts them back through the Client API.

## Key Patterns

- **SansIO**: `MeshNodeFSM` never calls WebRTC APIs directly; it only returns outputs for the Driver to act on. This makes protocol logic unit-testable without a browser.
- **Generic message type**: All layers are generic over `Msg: UserMsgPayload`. The example uses `struct Message { text: String }`.
- **Callbacks**: `RtcCallbacks<Msg>` supports multiple subscribers per event type, keyed by subscription ID. Both Rust closures and JS callbacks (via `wasm-bindgen`) are supported.


