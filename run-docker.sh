#!/bin/bash
# Simple script to ensure .env exists and run docker-compose

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

if [ ! -f .env ]; then
    echo "📝 .env file not found, creating it..."
    if [ -f setup.sh ]; then
        chmod +x setup.sh
        bash setup.sh || echo "⚠️  Setup script had issues, but continuing..."
    elif [ -f .devcontainer/post-create.sh ]; then
        bash .devcontainer/post-create.sh
    else
        echo "❌ Cannot create .env - setup.sh or post-create.sh not found"
        exit 1
    fi
fi

if [ ! -s .env ]; then
    echo "❌ .env file is empty!"
    exit 1
fi

set -a
source .env
set +a

if docker compose version >/dev/null 2>&1; then
    DOCKER_COMPOSE_CMD="docker compose"
elif command -v docker-compose >/dev/null 2>&1; then
    DOCKER_COMPOSE_CMD="docker-compose"
else
    echo "❌ Neither 'docker compose' nor 'docker-compose' found"
    exit 1
fi

echo "🚀 Starting Docker services..."
$DOCKER_COMPOSE_CMD --env-file .env "$@"
