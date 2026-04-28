# Crate structure

## antenna

Фасадный crate, который пользователь подключает в свой проект. Реэкспортирует `antenna-protocol` и одну из платформенных реализаций по feature-флагу: `web` (WASM) или `native` (tokio). Опциональный feature `signaling-client` подтягивает встроенный сигналинг-клиент.

## protocol

SansIO-ядро протокола. Содержит `MeshNodeFSM` и весь handshake/relay/reconnect-автомат. Не знает ни про webRTC, ни про tokio/wasm — взаимодействует с миром через `Input`/`Output` сообщения. Это делает ядро тестируемым без сети и переносимым между платформами.

## client/shared

Общие платформонезависимые абстракции, переиспользуемые в `client/web` и `client/native`: типы событий и колбэков (`Event`, `RtcCallbacks`), конфигурация ICE-серверов (`IceServerConfig`), интерфейс persistent storage для identity, общие константы.

## client/web

Платформенная реализация для браузера. Компилируется в WebAssembly через `wasm-bindgen`, использует браузерный webRTC API через `web-sys`-биндинги. Сохраняет identity в `localStorage`, автоматически слушает `beforeunload` для graceful-выхода.

## client/native

Платформенная реализация для нативного rust. Построена на tokio-рантайме, использует crate `webrtc-rs` в качестве webRTC-стека. Identity хранится в файле, путь к которому передаётся через `Storage`.

## signaling-server

Референс-реализация встроенного сигналинг-сервера на axum + tokio. WebSocket-эндпойнт, in-memory комнаты, без аутентификации.

## arbitrary-tests

Property-based тесты на `MeshNodeFSM` через [proptest](https://crates.io/crates/proptest). Гоняет случайные последовательности `Input`-сообщений и проверяет инварианты протокола (полнота меша, отсутствие зависших состояний, корректность статус-переходов).

## integration-tests

End-to-end интеграционные тесты с реальным webRTC-стеком (через `client/native`). Проверяют сценарии bootstrap'а пар, расширения меша, graceful leave, reconnect после force-drop. Запускаются через `cargo nextest` (см. `.config/nextest.toml`).

# Tests

Run tests with:

```
cargo nextest run
```
