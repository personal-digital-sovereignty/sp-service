# ============================================
# sp-service — Dockerfile (Runtime-Only)
# ============================================
# Receives the pre-compiled sovereign-daemon binary from CI.
# No Rust compilation happens here — binaries are built by
# the build-core matrix and injected via build context.
# ============================================

FROM ubuntu:24.04

ENV DEBIAN_FRONTEND=noninteractive

# Install runtime dependencies only
RUN apt-get update && apt-get install -y --no-install-recommends \
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

# Copy pre-compiled binary (architecture is resolved by Docker buildx TARGETARCH)
ARG TARGETARCH
COPY binaries/${TARGETARCH}/sovereign-daemon /app/sovereign-daemon
RUN chmod +x /app/sovereign-daemon

# Copy Python workers
COPY python_workers /app/python_workers

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
