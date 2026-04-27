## Antenna P2P SDK

Antenna is an SDK for building decentralized P2P over WebRTC. The SDK is designed to be as flexible as possible for the user,
utilizing the advantages of the webRTC protocol while fully encapsulating its API and handling common tasks such as
signaling and reconnection. Antenna is a cross-platform SDK and can be used in both browser-based and
native applications, although the focus is on the browser.

### Implemented features by platform:

| Feature | WASM | rust tokio |
| :--- | :---: | :---: |
| Peer implementation | ✅ | ✅ |
| Signaling client | ✅ | ✅ |

### Usage 

[See WASM example](https://github.com/PixelQuasar/antenna/tree/main/example/minimal-chat)

[See WASM example with signaling server](https://github.com/PixelQuasar/antenna/tree/main/example/chat-with-signaling-server)

[See bin example](https://github.com/PixelQuasar/antenna/tree/main/example/shell-chat)

#### Instantiate peer (native):
```rust
let peer = Arc::new(Peer::new(Storage::new("./antenna-identity.json")));
```


#### Subscribe on events:
```rust
peer.subscribe(Event::UserMessage(MessageCallback::<Message>::from_fn(
    |peer_id, msg| {
        println!("[{peer_id}] {}", msg.0);
        Ok(())
    },
)))
```


#### simple REPL example for performing handshake and broadcast messages:
```rust
while let Ok(Some(line)) = lines.next_line().await {
    let line = line.trim();
    if line.is_empty() {
        continue;
    }
    let (cmd, rest) = line.split_once(' ').unwrap_or((line, ""));
    match cmd {
        "start" => match peer.start().await {
            Ok(offer) => println!("offer: {offer}"),
            Err(e) => eprintln!("error: {e:#}"),
        },
        "offer" => match peer.receive_offer(rest).await {
            Ok(answer) => println!("answer: {answer}"),
            Err(e) => eprintln!("error: {e:#}"),
        },
        "answer" => {
            if let Err(e) = peer.receive_answer(rest).await {
                eprintln!("error: {e:#}");
            }
        }
        "send" => peer.broadcast(Message(rest.to_string())),
        "quit" => {
            peer.leave();
            break;
        }
        _ => println!("unknown command: {cmd}"),
    }
}
```


[Know more in docs](https://github.com/PixelQuasar/antenna/blob/main/docs/index.md)
