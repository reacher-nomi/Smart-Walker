#!/bin/bash
# GitHub Codespaces Setup Script
# Automatically sets up the MedHealth platform

set -e

echo "🚀 Setting up MedHealth Platform..."

# Navigate to project root
cd /workspace || cd "$(dirname "$0")/.."

# ==============================================
# 1. Generate .env file if it doesn't exist
# ==============================================
if [ ! -f .env ]; then
    echo "📝 Generating .env file..."
    
    # Check for GitHub Codespaces secrets first (available as environment variables)
    # If not set, generate secure random secrets
    if [ -n "$POSTGRES_PASSWORD" ]; then
        echo "   ✓ Using POSTGRES_PASSWORD from GitHub Codespaces secrets"
        POSTGRES_PASSWORD_VAL="$POSTGRES_PASSWORD"
    else
        echo "   🔑 Generating secure random POSTGRES_PASSWORD"
        POSTGRES_PASSWORD_VAL=$(openssl rand -base64 24 | tr -d '\n')
    fi
    
    if [ -n "$JWT_SECRET" ]; then
        echo "   ✓ Using JWT_SECRET from GitHub Codespaces secrets"
        JWT_SECRET_VAL="$JWT_SECRET"
    else
        echo "   🔑 Generating secure random JWT_SECRET"
        JWT_SECRET_VAL=$(openssl rand -base64 48 | tr -d '\n')
    fi
    
    if [ -n "$DEVICE_SECRET" ]; then
        echo "   ✓ Using DEVICE_SECRET from GitHub Codespaces secrets"
        DEVICE_SECRET_VAL="$DEVICE_SECRET"
    else
        echo "   🔑 Generating secure random DEVICE_SECRET"
        DEVICE_SECRET_VAL=$(openssl rand -base64 32 | tr -d '\n')
    fi
    
    if [ -n "$PGADMIN_DEFAULT_PASSWORD" ]; then
        echo "   ✓ Using PGADMIN_DEFAULT_PASSWORD from GitHub Codespaces secrets"
        PGADMIN_PASSWORD_VAL="$PGADMIN_DEFAULT_PASSWORD"
    else
        echo "   🔑 Generating secure random PGADMIN_DEFAULT_PASSWORD"
        PGADMIN_PASSWORD_VAL=$(openssl rand -base64 16 | tr -d '\n')
    fi
    
    # Use environment variables or defaults
    POSTGRES_USER_VAL="${POSTGRES_USER:-medhealth}"
    POSTGRES_DB_VAL="${POSTGRES_DB:-medhealth_db}"
    
    # Create .env file
    cat > .env << EOF
# MedHealth Environment Variables (Auto-generated)
# Generated on: $(date)
# Source: GitHub Codespaces secrets (if available) or auto-generated

# Database Configuration
POSTGRES_USER=${POSTGRES_USER_VAL}
POSTGRES_PASSWORD=${POSTGRES_PASSWORD_VAL}
POSTGRES_DB=${POSTGRES_DB_VAL}
POSTGRES_HOST=postgres
POSTGRES_PORT=5432
DATABASE_URL=postgresql://${POSTGRES_USER_VAL}:${POSTGRES_PASSWORD_VAL}@postgres:5432/${POSTGRES_DB_VAL}

# Redis Configuration
REDIS_URL=redis://redis:6379

# JWT Authentication (48-byte base64)
JWT_SECRET=${JWT_SECRET_VAL}

# Device HMAC Authentication (32-byte base64)
DEVICE_SECRET=${DEVICE_SECRET_VAL}

# pgAdmin
PGADMIN_DEFAULT_EMAIL=admin@medhealth.local
PGADMIN_DEFAULT_PASSWORD=${PGADMIN_PASSWORD_VAL}

# Server Configuration
SERVER_BIND_ADDR=0.0.0.0:8080
SERVER_WORKERS=4

# Logging
LOG_LEVEL=info
RUST_LOG=info

# CORS
CORS_ALLOWED_ORIGINS=http://localhost:5173,http://127.0.0.1:5173,http://localhost:3000
EOF

    echo "   ✓ .env file created"
    if [ -n "$POSTGRES_PASSWORD" ] || [ -n "$JWT_SECRET" ] || [ -n "$DEVICE_SECRET" ]; then
        echo "   ℹ️  Using secrets from GitHub Codespaces"
    else
        echo "   ℹ️  Using auto-generated secrets (stored in .env, gitignored)"
    fi
else
    echo "✓ .env file already exists"
fi

# ==============================================
# 2. Load environment variables
# ==============================================
if [ -f .env ]; then
    export $(cat .env | grep -v '^#' | xargs)
    echo "✓ Environment variables loaded"
fi

# ==============================================
# 3. Create config.toml from environment
# ==============================================
echo "📝 Creating config.toml from environment variables..."

mkdir -p website/backend/logs

cat > website/backend/config.toml << EOF
# MedHealth Backend Configuration
# Auto-generated from environment variables

[server]
bind_addr = "${SERVER_BIND_ADDR:-0.0.0.0:8080}"
workers = ${SERVER_WORKERS:-4}

[database]
url = "${DATABASE_URL}"
max_connections = 20
min_connections = 5

[redis]
url = "${REDIS_URL:-redis://redis:6379}"
pool_size = 10

[jwt]
secret = "${JWT_SECRET}"
expiration_hours = 24
refresh_token_days = 7

[cors]
allowed_origins = [$(echo $CORS_ALLOWED_ORIGINS | sed 's/,/", "/g' | sed 's/^/"/' | sed 's/$/"/')]

[device]
secret = "${DEVICE_SECRET}"
replay_window_seconds = 60

[ml]
anomaly_threshold = 0.85
enable_alerts = true
critical_hr_low = 40
critical_hr_high = 180
critical_spo2_low = 88

[fhir]
base_url = "http://localhost:8080/fhir"
organization_id = "org-medhealth-001"

[logging]
level = "${LOG_LEVEL:-info}"
audit_log_path = "./logs/audit.log"
enable_phi_encryption = true
EOF

echo "✓ config.toml created"

# ==============================================
# 4. Start docker-compose services
# ==============================================
echo ""
echo "🐳 Starting docker-compose services..."
cd /workspace/website/backend || cd "$(dirname "$0")/../website/backend"

# Load .env file for docker-compose
if [ -f /workspace/.env ]; then
    export $(cat /workspace/.env | grep -v '^#' | xargs)
fi

# Start services in background (postgres and redis)
echo "   Starting PostgreSQL and Redis..."
docker-compose up -d postgres redis

echo "   ⏳ Waiting for services to be healthy..."
sleep 5

# Check if services are running
if docker-compose ps | grep -q "medhealth_postgres.*Up"; then
    echo "   ✓ PostgreSQL is running"
else
    echo "   ⚠️  Warning: PostgreSQL may not be running"
fi

if docker-compose ps | grep -q "medhealth_redis.*Up"; then
    echo "   ✓ Redis is running"
else
    echo "   ⚠️  Warning: Redis may not be running"
fi

# ==============================================
# 5. Wait for services to be ready
# ==============================================
echo ""
echo "⏳ Waiting for PostgreSQL to be ready..."
until pg_isready -h postgres -U ${POSTGRES_USER:-medhealth} -d ${POSTGRES_DB:-medhealth_db} 2>/dev/null; do
    echo "   PostgreSQL is unavailable - sleeping"
    sleep 2
done
echo "✓ PostgreSQL is ready"

echo "⏳ Waiting for Redis to be ready..."
until redis-cli -h redis ping 2>/dev/null | grep -q PONG; do
    echo "   Redis is unavailable - sleeping"
    sleep 2
done
echo "✓ Redis is ready"

# ==============================================
# 6. Install Rust dependencies and run migrations
# ==============================================
cd website/backend

echo ""
echo "📦 Installing Rust dependencies..."
cargo fetch

echo "🗄️  Running database migrations..."
cargo install sqlx-cli --no-default-features --features postgres 2>/dev/null || true
sqlx migrate run || echo "⚠️  Migration failed (database may already be initialized)"

echo "🔨 Building backend..."
cargo build --release

echo ""
echo "✅ Setup complete!"
echo ""
echo "🎉 Platform Status:"
echo "   Services: docker-compose up -d (postgres & redis running)"
echo "   Backend: Ready to start with 'cargo run'"
echo "   API: Will be available at http://localhost:8080"
echo ""
echo "📋 Credentials (saved in .env):"
echo "   Database: postgres:5432 (user: ${POSTGRES_USER:-medhealth})"
echo "   Redis: redis:6379"
echo "   pgAdmin: http://localhost:5050 (email: admin@medhealth.local)"
echo ""
echo "💡 To start all services: cd website/backend && docker-compose up"
echo ""
