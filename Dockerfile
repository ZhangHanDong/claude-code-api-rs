# Build stage
FROM rust:1.75-alpine AS builder

RUN apk add --no-cache musl-dev

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

RUN apk add --no-cache ca-certificates

WORKDIR /app

# Copy binary from builder
COPY --from=builder /app/target/release/claude-code-api /usr/local/bin/claude-code-api

# Create non-root user
RUN addgroup -g 1000 claude && \
    adduser -D -u 1000 -G claude claude

USER claude

EXPOSE 8080

CMD ["claude-code-api"]