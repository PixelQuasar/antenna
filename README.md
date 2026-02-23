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
