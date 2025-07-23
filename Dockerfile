# Build stage
FROM rust:1.88.0-alpine3.20 AS builder

RUN apk add --no-cache musl-dev openssl-dev openssl-libs-static pkgconfig

WORKDIR /app

# Copy manifests
COPY Cargo.toml ./

# Build dependencies - this is the caching layer
RUN mkdir src && \
    echo "fn main() {}" > src/main.rs && \
    cargo build --release && \
    rm -rf src

# Copy source code
COPY src ./src

# Build application
RUN touch src/main.rs && \
    cargo build --release

# Runtime stage
FROM alpine:latest

# Install runtime dependencies including Node.js and npm
RUN apk add --no-cache ca-certificates nodejs npm

WORKDIR /app

# Copy binary from builder
COPY --from=builder /app/target/release/claude-code-api /usr/local/bin/claude-code-api

# Install Claude CLI globally
RUN npm install -g @anthropic-ai/claude-code

# Create non-root user
RUN addgroup -g 1000 claude && \
    adduser -D -u 1000 -G claude claude

# Create necessary directories and set permissions
RUN mkdir -p /home/claude/.config /home/claude/.claude && \
    chown -R claude:claude /home/claude

# Copy .claude directory with settings
COPY --chown=claude:claude .claude /home/claude/.claude

USER claude

EXPOSE 8080

CMD ["claude-code-api"]