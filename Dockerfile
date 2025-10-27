# --- Stage 1: Build ---
FROM rust:bullseye AS builder

ENV PORT=8080
WORKDIR /app

# Install required build dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
 && rm -rf /var/lib/apt/lists/*

# Copy and build
COPY . .
RUN cargo build --release --locked

# --- Stage 2: Runtime Image ---
FROM debian:bullseye-slim

ENV PORT=8080
WORKDIR /app

# Install only minimal runtime dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
 && rm -rf /var/lib/apt/lists/*

# Copy the compiled binary from the builder stage
COPY --from=builder /app/target/release/x402-rs /usr/local/bin/x402-rs

EXPOSE $PORT
ENV RUST_LOG=info

ENTRYPOINT ["x402-rs"]