#!/bin/bash
set -e

echo "🐳 Starting Docker services..."

WORKSPACE_ROOT="$(cd "$(dirname "$0")/.." 2>/dev/null && pwd || echo "$(pwd)")"
cd "$WORKSPACE_ROOT"

echo "⏳ Waiting for Docker to be ready..."
timeout=60
docker_ready=false
while [ $timeout -gt 0 ]; do
    if docker info >/dev/null 2>&1 || sudo docker info >/dev/null 2>&1; then
        docker_ready=true
        echo "✅ Docker is ready"
        break
    fi
    if [ -S /var/run/docker.sock ] || [ -S /var/run/docker-host.sock ]; then
        docker_ready=true
        echo "✅ Docker socket found"
        break
    fi
    if [ $((timeout % 10)) -eq 0 ]; then
        echo "   Waiting... ($timeout seconds remaining)"
    fi
    sleep 1
    timeout=$((timeout - 1))
done

if [ "$docker_ready" = false ]; then
    echo "⚠️  Docker not ready, but continuing..."
    echo "ℹ️  You can manually start services with: docker-compose up -d"
    exit 0
fi

if [ -f .env ]; then
    set -a
    source .env
    set +a
fi

if [ -f docker-compose.yml ]; then
    echo "🚀 Starting docker-compose services..."
    if command -v docker-compose >/dev/null 2>&1; then
        docker-compose up -d || echo "⚠️  docker-compose failed, but continuing..."
    elif docker compose version >/dev/null 2>&1; then
        docker compose up -d || echo "⚠️  docker compose failed, but continuing..."
    else
        echo "⚠️  Neither docker-compose nor docker compose found"
        exit 0
    fi
    sleep 5
    echo "✅ Services started (or already running)"
else
    echo "⚠️  docker-compose.yml not found"
fi
