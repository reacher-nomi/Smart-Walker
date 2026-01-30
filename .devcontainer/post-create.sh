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

# Check if POSTGRES_PASSWORD is set via GitHub Secrets
if [ -z "$POSTGRES_PASSWORD" ]; then
    echo "⚠️  WARNING: POSTGRES_PASSWORD not set in GitHub Secrets"
    echo "   Generating a random password for this session..."
    echo "   For production, set POSTGRES_PASSWORD in GitHub Repository Secrets"
    export POSTGRES_PASSWORD=$(generate_secret)
fi

# Generate JWT secret if not provided via GitHub Secrets
if [ -z "$JWT_SECRET" ] && [ -z "$MEDHEALTH__JWT__SECRET" ]; then
    echo "🔑 Generating secure random JWT_SECRET"
    export MEDHEALTH__JWT__SECRET=$(generate_secret)
else
    export MEDHEALTH__JWT__SECRET="${JWT_SECRET:-$MEDHEALTH__JWT__SECRET}"
fi

# Generate device secret if not provided via GitHub Secrets
if [ -z "$DEVICE_SECRET" ] && [ -z "$MEDHEALTH__DEVICE__SECRET" ]; then
    echo "🔑 Generating secure random DEVICE_SECRET"
    export MEDHEALTH__DEVICE__SECRET=$(generate_secret)
else
    export MEDHEALTH__DEVICE__SECRET="${DEVICE_SECRET:-$MEDHEALTH__DEVICE__SECRET}"
fi

echo "📝 Creating .env file from GitHub Secrets + generated values..."
cat > .env << ENVEOF
# Auto-generated from GitHub Codespaces Secrets
# DO NOT commit this file to version control!
POSTGRES_USER=${POSTGRES_USER:-medhealth}
POSTGRES_PASSWORD=${POSTGRES_PASSWORD}
POSTGRES_DB=${POSTGRES_DB:-medhealth_db}
POSTGRES_PORT=${POSTGRES_PORT:-5433}
REDIS_PORT=${REDIS_PORT:-6379}
JWT_SECRET=${MEDHEALTH__JWT__SECRET}
DEVICE_SECRET=${MEDHEALTH__DEVICE__SECRET}
ENVEOF

echo "✅ .env file created (using GitHub Secrets where available)"

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
