FROM rust:1.89 as wasm-builder

WORKDIR /app

COPY . .

# Install wasm32 target
RUN rustup target add wasm32-unknown-unknown

# Install wasm-pack
RUN curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh

# Build antenna-cli
RUN cargo build --release -p antenna-cli

# Run antenna-cli to generate bindings and build wasm
WORKDIR /app/examples/simple-group-chat
RUN /app/target/release/cargo-antenna antenna build --shared ./shared --client ./wasm-lib --out ./client/src/generated

# Now build the frontend
FROM node:20 as frontend-builder

WORKDIR /app/client

# Copy package.json and package-lock.json
COPY examples/simple-group-chat/client/package*.json ./
RUN npm install

# Copy the client source code
COPY examples/simple-group-chat/client .

# Copy the generated bindings and wasm from wasm-builder
COPY --from=wasm-builder /app/examples/simple-group-chat/client/src/generated ./src/generated

# Build the frontend
RUN npm run build

# Serve
RUN npm install -g serve
CMD ["serve", "-s", "dist", "-l", "5173"]
