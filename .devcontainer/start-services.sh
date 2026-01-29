#!/bin/bash
set -e

echo "🐳 Starting Docker services..."

# Ensure we're in the workspace root
WORKSPACE_ROOT="$(cd "$(dirname "$0")/.." 2>/dev/null && pwd || echo "$(pwd)")"
cd "$WORKSPACE_ROOT"

# Wait for Docker to be ready
echo "⏳ Waiting for Docker to be ready..."
timeout=30
while [ $timeout -gt 0 ]; do
    if docker info >/dev/null 2>&1; then
        echo "✅ Docker is ready"
        break
    fi
    echo "   Waiting... ($timeout seconds remaining)"
    sleep 1
    timeout=$((timeout - 1))
done

if ! docker info >/dev/null 2>&1; then
    echo "⚠️  Docker not ready, skipping docker-compose"
    exit 0
fi

# Load environment variables if .env exists
if [ -f .env ]; then
    set -a
    source .env
    set +a
fi

# Start services
if [ -f docker-compose.yml ]; then
    echo "🚀 Starting docker-compose services..."
    docker-compose up -d || echo "⚠️  docker-compose failed, but continuing..."
    
    # Wait for services to be healthy
    echo "⏳ Waiting for services to start..."
    sleep 5
    
    echo "✅ Services started (or already running)"
else
    echo "⚠️  docker-compose.yml not found"
fi
