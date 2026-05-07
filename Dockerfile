# ============================================
# sp-service — Dockerfile
# ============================================
# Containerização do backend Rust para produção
# ============================================

# --------------------------------------------
# Stage 1: Build (Rust)
# --------------------------------------------
FROM ubuntu:24.04 AS builder

ENV DEBIAN_FRONTEND=noninteractive

# Install dependencies
RUN apt-get update && apt-get install -y \
    curl \
    build-essential \
    cmake \
    pkg-config \
    libssl-dev \
    python3 \
    python3-pip \
    python3-venv \
    git \
    && rm -rf /var/lib/apt/lists/*

# Install Rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

# Set working directory
WORKDIR /app

# Copy manifests first (for better caching)
COPY Cargo.toml Cargo.lock ./
COPY build.rs ./
COPY prompts ./prompts
COPY python_workers ./python_workers
COPY src ./src
COPY tests ./tests

# Build in release mode
RUN cargo build --release

# --------------------------------------------
# Stage 2: Runtime
# --------------------------------------------
FROM ubuntu:24.04 AS runtime

ENV DEBIAN_FRONTEND=noninteractive

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    openssl \
    python3 \
    python3-pip \
    python3-venv \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user (remove default ubuntu user first since it occupies UID 1000)
RUN id -u ubuntu > /dev/null 2>&1 && userdel -r ubuntu || true; \
    useradd -m -u 1000 -r sovereign

# Set working directory
WORKDIR /app

# Copy binary from builder
COPY --from=builder /app/target/release/sovereign-daemon /app/sovereign-daemon

# Copy Python workers
COPY --from=builder /app/python_workers /app/python_workers

# Create directories for data
RUN mkdir -p /app/data /app/data/vault && \
    chown -R sovereign:sovereign /app

# Switch to non-root user
USER sovereign

# Expose port
EXPOSE 8080

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

# Set environment variables
ENV RUST_LOG=info
ENV DATABASE_PATH=/app/data/sensus_nexus.db
ENV WORKSPACE_PATH=/app/data/vault
ENV OLLAMA_HOST=host.docker.internal:11434

# Run the service
CMD ["./sovereign-daemon"]
