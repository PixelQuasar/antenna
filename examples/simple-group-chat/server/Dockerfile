FROM rust:1.89 as builder

WORKDIR /app

COPY . .

WORKDIR /app/examples/simple-group-chat/server

RUN cargo build --release

FROM debian:bookworm-slim

WORKDIR /app

COPY --from=builder /app/examples/simple-group-chat/target/release/server .

CMD ["./server"]
