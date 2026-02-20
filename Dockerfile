# ---------------- BUILD STAGE ----------------
FROM rust:slim-bookworm as builder

WORKDIR /app

RUN apt-get update && \
    apt-get install -y pkg-config libssl-dev && \
    rm -rf /var/lib/apt/lists/*

# Copy cargo files first (cache deps)
COPY Cargo.toml Cargo.lock ./

# Dummy build to cache dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release

# Remove dummy src
RUN rm -rf src

# NOW copy real source AND migrations
COPY src ./src
COPY migrations ./migrations

RUN ls -la

# Build real app
RUN cargo build --release

# Runtime Stage
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && \
    apt-get install -y libssl3 ca-certificates postgresql-client && \
    rm -rf /var/lib/apt/lists/*

# Copy binary from builder
COPY --from=builder /app/target/release/backend ./backend

# Copy migrations
COPY migrations ./migrations

EXPOSE 3000

# Run migrations and then start the app
CMD ./backend