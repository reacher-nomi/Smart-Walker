# Multi-stage Rust build for MedHealth Backend
# Produces a small, optimized production image

# ==============================================
# Stage 1: Build Stage
# ==============================================
FROM rust:1.75-slim as builder

# Install required dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Create app directory
WORKDIR /app

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Create dummy main.rs to cache dependencies
RUN mkdir src && \
    echo "fn main() {}" > src/main.rs && \
    cargo build --release && \
    rm -rf src

# Copy actual source code
COPY src ./src
COPY migrations ./migrations

# Build the application
RUN cargo build --release

# ==============================================
# Stage 2: Runtime Stage
# ==============================================
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -m -u 1000 medhealth

# Set working directory
WORKDIR /app

# Copy binary from builder
COPY --from=builder /app/target/release/medhealth-backend /app/

# Copy migrations
COPY --from=builder /app/migrations /app/migrations

# Create logs directory
RUN mkdir -p /app/logs && chown medhealth:medhealth /app/logs

# Switch to non-root user
USER medhealth

# Expose port
EXPOSE 8080

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

# Run the binary
CMD ["/app/medhealth-backend"]
