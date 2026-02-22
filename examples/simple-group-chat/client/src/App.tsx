import React, {useEffect, useRef, useState} from 'react';

import init, {ChatWrapper} from './generated/wasm';

import type {ChatClientMsg} from "./generated/types/ChatClientMsg";
import type {ChatServerMsg} from "./generated/types/ChatServerMsg";

const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
const SERVER_URL = import.meta.env.VITE_SERVER_URL || `${protocol}//${window.location.host}/ws`;
const AUTH_TOKEN = "test-token-123";

function App() {
    const [isReady, setIsReady] = useState(false);
    const [error, setError] = useState<string | null>(null);
    const [inputText, setInputText] = useState("");

    const [messages, setMessages] = useState<ChatServerMsg[]>([]);

    const chatRef = useRef<ChatWrapper | null>(null);
    const runOnce = useRef(false);


    useEffect(() => {
        if (runOnce.current) return;
        runOnce.current = true;

        const startWasm = async () => {
            try {
                await init();
                const userId = self.crypto.randomUUID();

                const url = `${SERVER_URL}/${userId}`;

                console.log(`Connecting to ${url}...`);
                
                // TODO: Replace with your own TURN server credentials
                const iceServers = [
                    {
                        urls: ["stun:stun.l.google.com:19302"],
                    },
                    // Example TURN server configuration:
                    /*
                    {
                        urls: ["turn:global.turn.metered.ca:80"],
                        username: "your_username",
                        credential: "your_password"
                    }
                    */
                ];

                const client = new ChatWrapper(url, AUTH_TOKEN, iceServers);

                client.on_event((rawEvent: any) => {
                    const event = rawEvent as ChatServerMsg;
                    console.log("Received:", event);
                    setMessages((prev) => [...prev, event]);
                });

                chatRef.current = client;
                setIsReady(true);
            } catch (err: any) {
                console.error("WASM Error:", err);
                setError(err.toString());
            }
        };

        startWasm();

        return () => {
            chatRef.current?.free();
        };
    }, []);

    const handleSend = () => {
        if (!chatRef.current || !inputText.trim()) return;

        const payload: ChatClientMsg = {
            text: inputText
        };

        try {
            chatRef.current.send_message(payload.text);
            setInputText("");
        } catch (e) {
            console.error("Send error:", e);
        }
    };

    const handleKeyDown = (e: React.KeyboardEvent) => {
        if (e.key === 'Enter') handleSend();
    };

    if (error) {
        return <div className="error">Error: {error}</div>;
    }

    if (!isReady) {
        return <div className="loading">Loading AntennaEngine...</div>;
    }

    return (
        <div className="chat-container" style={{padding: '20px', maxWidth: '600px', margin: '0 auto'}}>
            <h1>Antenna Chat 📡</h1>

            <div className="messages-list" style={{
                border: '1px solid #ccc',
                borderRadius: '8px',
                height: '400px',
                overflowY: 'auto',
                padding: '10px',
                marginBottom: '10px',
                display: 'flex',
                flexDirection: 'column',
                gap: '8px'
            }}>
                {messages.length === 0 && <p style={{color: '#888'}}>No messages yet...</p>}

                {messages.map((msg, idx) => (
                    <div key={idx} className="message-item" style={{
                        background: '#f1f1f1',
                        padding: '8px',
                        borderRadius: '6px'
                    }}>
                        <div style={{fontWeight: 'bold', fontSize: '0.8em', color: '#555'}}>
                            {msg.author_id} <span
                            style={{fontWeight: 'normal'}}>at {new Date(Number(msg.timestamp)).toLocaleTimeString()}</span>
                        </div>
                        <div>{msg.text}</div>
                    </div>
                ))}
            </div>

            <div className="input-area" style={{display: 'flex', gap: '10px'}}>
                <input
                    type="text"
                    value={inputText}
                    onChange={(e) => setInputText(e.target.value)}
                    onKeyDown={handleKeyDown}
                    placeholder="Type your message..."
                    style={{flex: 1, padding: '10px', borderRadius: '4px', border: '1px solid #ccc'}}
                />
                <button
                    onClick={handleSend}
                    style={{
                        padding: '10px 20px',
                        cursor: 'pointer',
                        background: '#007bff',
                        color: '#fff',
                        border: 'none',
                        borderRadius: '4px'
                    }}
                >
                    Send
                </button>
            </div>
        </div>
    );
}

export default App;
