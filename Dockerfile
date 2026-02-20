# Build Stage
FROM rust:slim-bookworm as builder

WORKDIR /app

# Install build dependencies (needed for native-tls)
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

# Copy dependency files first to cache dependencies
COPY Cargo.toml Cargo.lock ./

# Create dummy main.rs to build dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release

# Remove dummy build and copy actual source
RUN rm -rf src
COPY src ./src

# Touch main.rs to ensure rebuild
# We need to touch the main source file to force a rebuild of the application code
# Since we just copied it over the dummy main.rs, the mtime might be older than the build artifacts
RUN touch src/main.rs
RUN cargo build --release

# Runtime Stage
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y libssl3 ca-certificates postgresql-client curl && rm -rf /var/lib/apt/lists/*

# Install sqlx-cli
RUN curl -L https://github.com/launchbadge/sqlx-cli/releases/download/v0.7.3/sqlx-cli-v0.7.3-x86_64-unknown-linux-musl.tar.gz | tar xz -C /usr/local/bin

# Copy binary from builder
COPY --from=builder /app/target/release/backend ./backend

# Copy migrations
COPY migrations ./migrations

EXPOSE 3000

# Run migrations and then start the app
CMD sh -c "sqlx migrate run && ./backend"
