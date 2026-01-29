#!/bin/bash
set -e

echo "🚀 Running post-create setup..."

# Ensure we're in the workspace root
cd "$(dirname "$0")/.." || cd /workspaces/Smart-Walker || pwd

# Function to generate a random secret
generate_secret() {
    openssl rand -hex 32 2>/dev/null || openssl rand -base64 32 | tr -d "=+/" | cut -c1-32
}

# Generate secrets if not provided
if [ -z "$POSTGRES_PASSWORD" ]; then
    echo "🔑 Generating secure random POSTGRES_PASSWORD"
    export POSTGRES_PASSWORD=$(generate_secret)
fi

if [ -z "$PGADMIN_PASSWORD" ]; then
    echo "🔑 Generating secure random PGADMIN_PASSWORD"
    export PGADMIN_PASSWORD=$(generate_secret)
fi

if [ -z "$MEDHEALTH__JWT__SECRET" ]; then
    echo "🔑 Generating secure random MEDHEALTH__JWT__SECRET"
    export MEDHEALTH__JWT__SECRET=$(generate_secret)
fi

if [ -z "$MEDHEALTH__DEVICE__SECRET" ]; then
    echo "🔑 Generating secure random MEDHEALTH__DEVICE__SECRET"
    export MEDHEALTH__DEVICE__SECRET=$(generate_secret)
fi

# Create .env file in root directory
echo "📝 Creating .env file..."
cat > .env << ENVEOF
POSTGRES_USER=${POSTGRES_USER:-medhealth}
POSTGRES_PASSWORD=${POSTGRES_PASSWORD}
POSTGRES_DB=${POSTGRES_DB:-medhealth_db}
POSTGRES_PORT=${POSTGRES_PORT:-5433}
REDIS_PORT=${REDIS_PORT:-6379}
PGADMIN_EMAIL=${PGADMIN_EMAIL:-admin@medhealth.local}
PGADMIN_PASSWORD=${PGADMIN_PASSWORD}
PGADMIN_PORT=${PGADMIN_PORT:-5050}
JWT_SECRET=${MEDHEALTH__JWT__SECRET}
DEVICE_SECRET=${MEDHEALTH__DEVICE__SECRET}
ENVEOF

echo "✅ .env file created with all required variables"

# Also create in backend directory if it exists
if [ -d "website/backend" ]; then
    cp .env website/backend/.env
    echo "✅ Backend .env file created"
fi

echo "✅ Post-create setup complete! You can now run 'docker-compose up'"
