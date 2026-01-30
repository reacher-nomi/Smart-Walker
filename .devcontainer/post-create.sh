#!/bin/bash
set -e

echo "🚀 Running post-create setup..."

WORKSPACE_ROOT="$(cd "$(dirname "$0")/.." 2>/dev/null && pwd || echo "$(pwd)")"
cd "$WORKSPACE_ROOT"

generate_secret() {
    if command -v openssl >/dev/null 2>&1; then
        openssl rand -hex 32 2>/dev/null
    elif [ -c /dev/urandom ]; then
        head -c 32 /dev/urandom | base64 | tr -d "=+/" | cut -c1-32
    elif command -v shuf >/dev/null 2>&1; then
        cat /dev/urandom | tr -dc 'a-zA-Z0-9' | fold -w 32 | head -n 1
    else
        echo "$(date +%s%N)$(cat /proc/sys/kernel/random/uuid 2>/dev/null || echo $$)" | sha256sum | cut -d' ' -f1 | cut -c1-32
    fi
}

# Use fixed default so backend and postgres volume always match (docker-compose default is "changeme")
if [ -z "$POSTGRES_PASSWORD" ]; then
    export POSTGRES_PASSWORD="${POSTGRES_PASSWORD:-changeme}"
fi

if [ -z "$MEDHEALTH__JWT__SECRET" ]; then
    echo "🔑 Generating secure random MEDHEALTH__JWT__SECRET"
    export MEDHEALTH__JWT__SECRET=$(generate_secret)
fi

if [ -z "$MEDHEALTH__DEVICE__SECRET" ]; then
    echo "🔑 Generating secure random MEDHEALTH__DEVICE__SECRET"
    export MEDHEALTH__DEVICE__SECRET=$(generate_secret)
fi

echo "📝 Creating .env file..."
cat > .env << ENVEOF
POSTGRES_USER=${POSTGRES_USER:-medhealth}
POSTGRES_PASSWORD=${POSTGRES_PASSWORD}
POSTGRES_DB=${POSTGRES_DB:-medhealth_db}
POSTGRES_PORT=${POSTGRES_PORT:-5433}
REDIS_PORT=${REDIS_PORT:-6379}
JWT_SECRET=${MEDHEALTH__JWT__SECRET}
DEVICE_SECRET=${MEDHEALTH__DEVICE__SECRET}
ENVEOF

echo "✅ .env file created"

if [ -d "website/backend" ]; then
    cp .env website/backend/.env 2>/dev/null || true
    echo "✅ Backend .env file created"
fi

echo "✅ Post-create setup complete!"

# Verify .env file was created and has content
if [ -f .env ]; then
    echo "✅ Verified .env file exists"
    echo "   File size: $(wc -l < .env) lines"
else
    echo "❌ ERROR: .env file was not created!"
    exit 1
fi
