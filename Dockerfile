# Stage 1: Build Rust binary
FROM rust:1.83-slim AS rust-builder

RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

WORKDIR /build

# Cache dependencies — copy manifests first
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/

RUN cargo build --release --bin argus-server

# Stage 2: Build Next.js frontend
FROM node:20-slim AS frontend-builder

WORKDIR /build/frontend

COPY frontend/package.json frontend/package-lock.json ./
RUN npm ci

COPY frontend/ ./
RUN npm run build

# Stage 3: Runtime
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the compiled binary
COPY --from=rust-builder /build/target/release/argus-server /app/argus-server

# Copy the frontend build
COPY --from=frontend-builder /build/frontend/.next /app/frontend/.next
COPY --from=frontend-builder /build/frontend/public /app/frontend/public
COPY --from=frontend-builder /build/frontend/package.json /app/frontend/package.json

EXPOSE 8080

CMD ["/app/argus-server"]
