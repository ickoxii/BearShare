# Dockerfile for Collaborative Editor Server
# Multi-stage build for optimal size

# Stage 1: Build
FROM rust:1.88 AS builder

WORKDIR /build

# Copy RGA dependency first (assumes build context includes both projects)
COPY Cargo.toml ./Cargo.toml

COPY crates ./crates

# Build for release
RUN cargo build --package server

# Stage 2: Runtime
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && \
    apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy binary from builder
COPY --from=builder /build/target/debug/server .

# Create directory for file store
RUN mkdir -p /app/file_store

# Expose WebSocket port
EXPOSE 9001

# Set environment variables
ENV BIND_ADDR="0.0.0.0:9001"
ENV FILE_STORE_PATH="/app/file_store"
ENV RUST_LOG="info,server=debug"

# Run the server
CMD ["./server"]
