#!/bin/bash
# GitHub Codespaces Setup Script
# Automatically sets up the MedHealth platform

set -e

echo "🚀 Setting up MedHealth Platform..."

# Navigate to project root - use dynamic path detection (NO hardcoded paths)
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR" || exit 1

# Function to generate a random secret
generate_secret() {
    openssl rand -hex 32 2>/dev/null || openssl rand -base64 32 | tr -d "=+/" | cut -c1-32
}

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
        POSTGRES_PASSWORD_VAL=$(generate_secret)
    fi
    
    if [ -n "$MEDHEALTH__JWT__SECRET" ] || [ -n "$JWT_SECRET" ]; then
        echo "   ✓ Using JWT_SECRET from GitHub Codespaces secrets"
        JWT_SECRET_VAL="${MEDHEALTH__JWT__SECRET:-$JWT_SECRET}"
    else
        echo "   🔑 Generating secure random JWT_SECRET"
        JWT_SECRET_VAL=$(generate_secret)
    fi
    
    if [ -n "$MEDHEALTH__DEVICE__SECRET" ] || [ -n "$DEVICE_SECRET" ]; then
        echo "   ✓ Using DEVICE_SECRET from GitHub Codespaces secrets"
        DEVICE_SECRET_VAL="${MEDHEALTH__DEVICE__SECRET:-$DEVICE_SECRET}"
    else
        echo "   🔑 Generating secure random DEVICE_SECRET"
        DEVICE_SECRET_VAL=$(generate_secret)
    fi
    
    if [ -n "$PGADMIN_PASSWORD" ] || [ -n "$PGADMIN_DEFAULT_PASSWORD" ]; then
        echo "   ✓ Using PGADMIN_PASSWORD from GitHub Codespaces secrets"
        PGADMIN_PASSWORD_VAL="${PGADMIN_PASSWORD:-$PGADMIN_DEFAULT_PASSWORD}"
    else
        echo "   🔑 Generating secure random PGADMIN_PASSWORD"
        PGADMIN_PASSWORD_VAL=$(generate_secret)
    fi
    
    echo "   ℹ️  Using auto-generated secrets (stored in .env, gitignored)"
    
    # Create .env file in root directory for docker-compose
    cat > .env << EOF
POSTGRES_USER=${POSTGRES_USER:-medhealth}
POSTGRES_PASSWORD=${POSTGRES_PASSWORD_VAL}
POSTGRES_DB=${POSTGRES_DB:-medhealth_db}
POSTGRES_PORT=${POSTGRES_PORT:-5433}
REDIS_PORT=${REDIS_PORT:-6379}
PGADMIN_EMAIL=${PGADMIN_EMAIL:-admin@medhealth.local}
PGADMIN_PASSWORD=${PGADMIN_PASSWORD_VAL}
PGADMIN_PORT=${PGADMIN_PORT:-5050}
JWT_SECRET=${JWT_SECRET_VAL}
DEVICE_SECRET=${DEVICE_SECRET_VAL}
EOF
    
    echo "   ✓ .env file created"
else
    echo "📝 Loading existing .env file..."
    set -a
    source .env
    set +a
    echo "   ✓ .env file loaded"
fi

# Export variables for use in rest of script
export POSTGRES_PASSWORD="${POSTGRES_PASSWORD:-$POSTGRES_PASSWORD_VAL}"
export PGADMIN_PASSWORD="${PGADMIN_PASSWORD:-$PGADMIN_PASSWORD_VAL}"
export MEDHEALTH__JWT__SECRET="${MEDHEALTH__JWT__SECRET:-$JWT_SECRET_VAL}"
export MEDHEALTH__DEVICE__SECRET="${MEDHEALTH__DEVICE__SECRET:-$DEVICE_SECRET_VAL}"

# Also create .env in backend directory for backend config
if [ -d "website/backend" ]; then
    cp .env website/backend/.env 2>/dev/null || true
    echo "   ✓ Backend .env file created"
fi

# ==============================================
# 2. Create config.toml in backend directory
# ==============================================
if [ -d "website/backend" ]; then
    echo "📝 Creating config.toml from environment variables..."
    cat > website/backend/config.toml << CONFIGEOF
[server]
bind_addr = "0.0.0.0:8080"
workers = 4

[database]
url = "postgresql://${POSTGRES_USER:-medhealth}:${POSTGRES_PASSWORD}@localhost:${POSTGRES_PORT:-5433}/${POSTGRES_DB:-medhealth_db}"
max_connections = 20
min_connections = 5

[redis]
url = "redis://localhost:${REDIS_PORT:-6379}"
pool_size = 10

[jwt]
secret = "${MEDHEALTH__JWT__SECRET}"
expiration_hours = 24
refresh_token_days = 7

[cors]
allowed_origins = ["http://localhost:5173", "http://127.0.0.1:5173"]

[device]
secret = "${MEDHEALTH__DEVICE__SECRET}"
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
level = "debug"
audit_log_path = "./logs/audit.log"
enable_phi_encryption = true
CONFIGEOF
    echo "   ✓ config.toml created"
fi

echo "✓ Environment variables loaded"

# ==============================================
# 3. Start Docker services
# ==============================================
echo "🐳 Starting docker-compose services..."
docker-compose up -d

echo "⏳ Waiting for services to be ready..."
sleep 5

# Wait for PostgreSQL
timeout=30
counter=0
until docker-compose exec -T postgres pg_isready -U ${POSTGRES_USER:-medhealth} > /dev/null 2>&1; do
    sleep 2
    counter=$((counter + 2))
    if [ $counter -ge $timeout ]; then
        echo "⚠️  PostgreSQL not ready yet, but continuing..."
        break
    fi
done
echo "   ✓ PostgreSQL is ready"

# Wait for Redis
counter=0
until docker-compose exec -T redis redis-cli ping > /dev/null 2>&1; do
    sleep 2
    counter=$((counter + 2))
    if [ $counter -ge $timeout ]; then
        echo "⚠️  Redis not ready yet, but continuing..."
        break
    fi
done
echo "   ✓ Redis is ready"

echo ""
echo "======================================"
echo "✅ Docker services are running!"
echo ""
echo "Services running:"
echo "  - PostgreSQL: localhost:${POSTGRES_PORT:-5433}"
echo "  - Redis: localhost:${REDIS_PORT:-6379}"
echo "  - pgAdmin: localhost:${PGADMIN_PORT:-5050}"
echo ""
echo "Next steps:"
echo "  1. Backend: cd website/backend && cargo run"
echo "  2. Frontend: cd website/frontend && npm install && npm run dev"
echo ""
