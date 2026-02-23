## Architecture and modules

### Crate graph
```mermaid
graph TD
    subgraph "antenna-core"
        Core["antenna-core<br/>(Shared Types, Traits, Protocol)"]
    end

    subgraph "antenna-server"
        Server["antenna-server (Signaling, SFU, Room Management)"]
        Server -->|Uses| Core
        Server -->|Uses| Axum["Axum (WebSocket)"]
        Server -->|Uses| WebRTC_RS["webrtc-rs"]
    end

    subgraph "antenna-wasm-gen"
        WasmGen["antenna-wasm-gen<br/>(WASM Bindings, WebRTC Logic)"]
        WasmGen -->|Uses| Core
        WasmGen -->|Uses| WebSys["web-sys (Browser API)"]
        
        
        TS_Wrapper -->|Wraps| WasmGen
    end

    subgraph "antenna-codegen"
        Codegen["antenna-codegen (currently: macros for TS Wrappers)"]
        
    end

    subgraph "antenna-cli"
        CLI["Provides cli for building wasm and generating TS types for client"]
    end

    subgraph "antenna facade"
        Antenna["antenna<br/>(Main Crate)"]
        Antenna -->|Re-exports| Server
        Antenna -->|Re-exports| WasmGen
        Antenna -->|Re-exports| Codegen
    end

```

### Signaling

The signaling process in Antenna is designed to establish WebRTC connections between peers via a central server. It handles the exchange of SDP offers/answers and ICE candidates.

#### Connection Flow

1.  **WebSocket Connection**: The client connects to the server via WebSocket.
2.  **Join Room**: The client sends a request to join a specific room.
3.  **WebRTC Negotiation (SDP Exchange)**:
    *   This process establishes the parameters for the media session (codecs, encryption, etc.).
    *   The server (acting as an SFU) creates an **SDP Offer** and sends it to the client.
    *   The client processes the offer and responds with an **SDP Answer**.
4.  **ICE Candidate Exchange**: Both parties exchange ICE candidates (network paths) to establish connectivity.
5.  **Media Exchange**: Once connected, media tracks (audio/video) are flowed through the server.

```mermaid
sequenceDiagram
    participant A as Client A
    participant Server as Antenna Server
    participant B as Client B

    Note over A, Server: WebSocket Connection Established
    Note over B, Server: WebSocket Connection Established

    A->>Server: Join Room "Lobby"
    Server-->>A: Room Joined (Success)

    Note over A, Server: WebRTC Negotiation (A)
    Server->>A: SDP Offer
    A->>Server: SDP Answer
    loop ICE Exchange
        Server->>A: ICE Candidate
        A->>Server: ICE Candidate
    end
    Note over A, Server: WebRTC Connected

    B->>Server: Join Room "Lobby"
    Server-->>B: Room Joined (Success)

    Note over B, Server: WebRTC Negotiation (B)
    Server->>B: SDP Offer
    B->>Server: SDP Answer
    loop ICE Exchange
        Server->>B: ICE Candidate
        B->>Server: ICE Candidate
    end
    Note over B, Server: WebRTC Connected
```

### Room logic 

Antenna server provides room management logic: each room runs in its own task, managing interactions of its peer connections.

#### Key Components

*   **RoomManager**: Maintains a registry of active rooms and handles the creation of new `Room` actors, creating `tokio::spawn` task for each new room and provides mpsc senders to signaling handler to `ws_handler`.

*   **Room**: The central unit for a group of user sessions.
    *   **RoomBehavior**: Developer-defined implementation.
    *   **Peers Data**: A map of connected peers and their data channels.
    *   **Transports**: Manages WebRTC connections for each peer.
    *   **Room Loop**: Listens for external commands from signaling and internal webRTC event like messages and disconnections

*   **RoomContext**: A handle passed to `RoomBehavior` methods, providing access to room operations. It allows sending messages to specific peers (`send`) or broadcasting to all (`broadcast`).

*   **ConnectionWrapper**: Encapsulates the `RTCPeerConnection`. It handles the complexity of WebRTC: managing tracks, processing ICE candidates, and bridging WebRTC events to the `Room` actor via `TransportEvent`.

```mermaid
graph TD
    subgraph "Room Actor"
        Room[Room Struct]
        Behavior[RoomBehavior Trait]
        Context[RoomContext]
        
        Room -->|Owns| Behavior
        Room -->|Creates| Context
        Behavior -->|Uses| Context
    end

    subgraph "Transport Layer"
        ConnWrapper[ConnectionWrapper]
        WebRTC[WebRTC PeerConnection]
        
        Room -->|Manages| ConnWrapper
        ConnWrapper -->|Wraps| WebRTC
    end

    subgraph "External"
        RoomManager[RoomManager]
        Client[Client / Peer]
    end

    RoomManager -->|Spawns| Room
    RoomManager -->|Routes Commands| Room
    
    Client -->|WebSocket| RoomManager
    Client <-->|WebRTC Media/Data| WebRTC

    ConnWrapper -->|TransportEvent| Room
    Room -->|Calls| Behavior
```

#### Data Flow

1.  **Signaling**: A `JoinRequest` is sent via WebSocket. `RoomManager` forwards it to the `Room`.
2.  **Connection**: The `Room` creates a `ConnectionWrapper`, negotiates SDP, and establishes the WebRTC connection.
3.  **Interaction**:
    *   When a peer sends data, `ConnectionWrapper` fires a `TransportEvent::Message`.
    *   The `Room` loop catches this event and calls `behavior.on_message(ctx, peer_id, data)`.
    *   The behavior implementation uses `ctx.broadcast(data)` to relay the message to other peers.



```mermaid
sequenceDiagram
    participant Client
    participant Transport as ConnectionWrapper
    participant Room as Room Actor
    participant Behavior as RoomBehavior
    participant Context as RoomContext

    Note over Client, Room: WebRTC Connection Established

    Client->>Transport: Send Data (DataChannel)
    Transport->>Room: TransportEvent::Message(peer_id, data)
    Room->>Behavior: on_message(ctx, peer_id, data)
    
    alt Broadcast Message
        Behavior->>Context: ctx.broadcast(data)
        loop For each peer
            Context->>Transport: channel.send(data)
            Transport->>Client: Data Received
        end
    end
```
This architecture ensures that business logic (`RoomBehavior`) is decoupled from the low-level WebRTC transport details (`ConnectionWrapper`), making it easy to build custom applications on top of Antenna.

### Client Logic and Antenna Engine

The client-side logic is primarily handled by the `antenna-wasm-gen` crate, which provides a Rust-based engine that compiles to WebAssembly. This engine manages the complexity of WebRTC and signaling, exposing a simplified API to the frontend (e.g., via TypeScript wrappers).

#### AntennaEngine

The `AntennaEngine<T, E>` struct is the core of the client implementation, where `T` is the type of messages sent to the server and `E` is the type of events received from the server.

*   **Initialization**:
    *   `new(config: EngineConfig)`: Initializes the engine with the signaling server URL, room ID, and optional ICE server configuration.
    *   `ws_setup`: Establishes the WebSocket connection to the signaling server.

*   **Signaling Handling**:
    *   The engine listens for WebSocket messages (`SignalMessage`) and dispatches them to appropriate handlers:
        *   `Welcome`: Triggers the connection initialization (`init_connection`).
        *   `Offer`: Handles an incoming SDP offer from the server (`handle_remote_offer`).
        *   `Answer`: Processes an SDP answer from the server.
        *   `IceCandidate`: Adds remote ICE candidates to the peer connection.

*   **WebRTC Management**:
    *   `create_pc`: Creates and configures the `RTCPeerConnection`.
    *   `init_connection`: Initiates the connection process (creating a data channel, creating an offer).
    *   `handle_remote_offer`: Responds to a server-initiated offer (e.g., when a new peer joins).

*   **Data Channel**:
    *   `setup_data_channel`: Configures the data channel for binary message exchange.
    *   `send(msg: T)`: Serializes and sends a message to the server via the data channel. If the channel is not open, messages are queued.
    *   `dispatch_event`: Deserializes incoming binary packets and invokes the registered JavaScript event handler.

*   **Media Handling**:
    *   `add_track`: Adds a local media track (audio/video) to the peer connection.
    *   `set_track_handler`: Registers a callback to handle incoming remote tracks.

#### Client Lifecycle

1.  **Setup**: The frontend creates an instance of `AntennaEngine`.
2.  **Connect**: The engine connects to the WebSocket signaling server.
3.  **Negotiation**:
    *   Upon receiving a `Welcome` message, the client initiates the WebRTC handshake.
    *   SDP offers and answers are exchanged via the WebSocket.
    *   ICE candidates are gathered and exchanged.
4.  **Active Session**:
    *   **Data**: Messages are sent and received via the `chat` data channel.
    *   **Media**: Tracks are added and received via the peer connection.
5.  **Events**: The engine triggers callbacks for received messages and new media tracks, allowing the frontend to update the UI.