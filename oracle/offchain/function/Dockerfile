# =============================================================================
# SSO-1 Switchboard Function Container
# =============================================================================
# This Dockerfile builds the TEE-compatible container for the SSO-1 signal
# oracle function. It is designed to run within AMD SEV-SNP enclaves via
# Switchboard V3 On-Demand infrastructure.
# =============================================================================

# -----------------------------------------------------------------------------
# Build Stage
# -----------------------------------------------------------------------------

FROM python:3.11-slim AS builder

WORKDIR /build

# Install build dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Copy and install Python dependencies
COPY requirements.txt .
RUN pip install --no-cache-dir --user -r requirements.txt

# -----------------------------------------------------------------------------
# Runtime Stage
# -----------------------------------------------------------------------------

FROM python:3.11-slim AS runtime

# Security: Run as non-root user
RUN useradd --create-home --shell /bin/bash sso && \
    mkdir -p /app && \
    chown -R sso:sso /app

WORKDIR /app

# Copy Python packages from builder
COPY --from=builder /root/.local /home/sso/.local
ENV PATH=/home/sso/.local/bin:$PATH

# Copy application code
COPY --chown=sso:sso . .

# Switch to non-root user
USER sso

# Environment variables (overridden at runtime)
ENV PYTHONUNBUFFERED=1
ENV LOG_LEVEL=info

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD python -c "import main; print('OK')" || exit 1

# Entry point for Switchboard function execution
ENTRYPOINT ["python", "main.py"]
