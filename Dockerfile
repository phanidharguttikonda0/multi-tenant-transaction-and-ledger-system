# Build Stage
FROM rust:1.91-slim-bookworm AS builder


WORKDIR /app

# 🔥 Install build dependencies (IMPORTANT)
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Copy dependency files
COPY Cargo.toml Cargo.lock ./

# Create a dummy main.rs to build dependencies and cache them
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release

# Remove the dummy binary and source
RUN rm -rf src

# Copy the actual source code
COPY . .

# Build the application
# We need to touch the main.rs to force a rebuild of the application package
RUN touch src/main.rs
RUN cargo build --release

# Runtime Stage
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies (OpenSSL, etc.)
RUN apt-get update && apt-get install -y \
    libssl-dev \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Copy the compiled binary from the builder
COPY --from=builder /app/target/release/multi-tenant-transaction-and-ledger-system /app/server
COPY --from=builder /app/migrations /app/migrations
COPY --from=builder /app/.env.example /app/.env.example

# Expose the application port
EXPOSE 4545

# Command to run the application
CMD ["/app/server"]
