FROM rust:1.85 as wasm-builder

WORKDIR /app

COPY . .

# Install wasm32 target
RUN rustup target add wasm32-unknown-unknown

# Build antenna-cli
RUN cargo build --release -p antenna-cli

# Run antenna-cli to generate bindings
WORKDIR /app/examples/simple-group-chat
RUN ../../target/release/antenna-cli build --shared ./shared --client ./wasm-lib --out ./client/src/generated

# Build the wasm library
WORKDIR /app/examples/simple-group-chat/wasm-lib
RUN cargo build --target wasm32-unknown-unknown --release

# Now build the frontend
FROM node:20 as frontend-builder

WORKDIR /app/client

# Copy package.json and package-lock.json
COPY examples/simple-group-chat/client/package*.json ./
RUN npm install

# Copy the client source code
COPY examples/simple-group-chat/client .

# Copy the generated bindings from wasm-builder
COPY --from=wasm-builder /app/examples/simple-group-chat/client/src/generated ./src/generated

# Copy the built wasm file to public directory, assuming the output name is wasm_lib.wasm based on crate name
COPY --from=wasm-builder /app/examples/simple-group-chat/target/wasm32-unknown-unknown/release/wasm_lib.wasm ./public/wasm_lib.wasm

# Build the frontend
RUN npm run build

# Serve
RUN npm install -g serve
CMD ["serve", "-s", "dist", "-l", "5173"]
