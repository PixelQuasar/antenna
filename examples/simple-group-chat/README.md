# Simple Group Chat

This example demonstrates a fullstack group chat application built with Antenna, featuring real-time messaging and voice calls.

## Application Architecture

This diagram illustrates how the application is built using Antenna, highlighting the separation between **User Code** and **Library Logic**.

```mermaid
graph TB
    subgraph "Client Side (Browser)"
        direction TB
        UI["User Frontend<br/>(React/JS/TS)"]
        
        subgraph "WASM Module"
            UserWrapper["User Wrapper<br/>(Rust struct with #[antenna_client])"]
            AntennaEngine["Antenna Engine<br/>(Library Logic)"]
        end
    end

    subgraph "Server Side"
        direction TB
        UserMain["User Server Entrypoint<br/>(main.rs)"]
        AntennaServer["Antenna Server<br/>(Library Core)"]
    end

    %% Protocol
    Protocol["Shared Protocol<br/>(Messages/Events)"]

    %% Flows
    UI -->|1. Calls Generated API| UserWrapper
    UserWrapper -->|2. Delegates to| AntennaEngine
    
    UserMain -->|3. Initializes & Runs| AntennaServer
    
    AntennaEngine <-->|"4. Signaling (WebSocket)"| AntennaServer
    AntennaEngine <-->|"5. Media & Data (WebRTC)"| AntennaServer

    Protocol -.->|Defines Types| UserWrapper
    Protocol -.->|Defines Types| UserMain
    
    %% Styling
    classDef user fill:#e1f5fe,stroke:#01579b,stroke-width:2px;
    classDef lib fill:#fff3e0,stroke:#e65100,stroke-width:2px;
    
    class UI,UserWrapper,UserMain,Protocol user;
    class AntennaEngine,AntennaServer lib;
```

## Running the Example

1.  **Start the Server:**
    ```bash
    cd server
    cargo run
    ```

2.  **Build the client engine via `antenna-cli`**
    ``` bash 
    cargo antenna build --shared ./shared --client ./wasm-lib --out ./client/src/generated
    ```

3.  **Start the Client:**
    ```bash
    cd client
    npm install
    npm run dev
    ```
