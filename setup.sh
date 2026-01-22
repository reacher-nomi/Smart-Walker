#!/bin/bash

# MedHealth Backend Setup Script
# This script automates the initial setup process

set -e

echo "🏥 MedHealth Backend Setup"
echo "======================================"
echo ""

# Check prerequisites
echo "📋 Checking prerequisites..."

# Check Rust
if ! command -v cargo &> /dev/null; then
    echo "❌ Rust not found. Install from https://rustup.rs/"
    exit 1
fi
echo "✅ Rust installed: $(rustc --version)"

# Check Docker
if ! command -v docker &> /dev/null; then
    echo "❌ Docker not found. Install from https://docs.docker.com/get-docker/"
    exit 1
fi
echo "✅ Docker installed: $(docker --version)"

# Check Docker Compose
if ! command -v docker-compose &> /dev/null; then
    echo "❌ Docker Compose not found"
    exit 1
fi
echo "✅ Docker Compose installed: $(docker-compose --version)"

echo ""
echo "📦 Starting infrastructure..."
cd website/backend

# Start PostgreSQL and Redis
docker-compose up -d

echo "⏳ Waiting for services to be ready..."
sleep 10

# Check if services are healthy
if ! docker-compose ps | grep -q "Up"; then
    echo "❌ Services failed to start. Check: docker-compose logs"
    exit 1
fi
echo "✅ PostgreSQL and Redis are running"

echo ""
echo "⚙️  Creating configuration..."

if [ ! -f "config.toml" ]; then
    cp config.example.toml config.toml
    echo "✅ Created config.toml (please edit secrets before production!)"
else
    echo "⚠️  config.toml already exists, skipping"
fi

echo ""
echo "🗄️  Setting up database..."

# Install sqlx-cli if not present
if ! command -v sqlx &> /dev/null; then
    echo "Installing sqlx-cli..."
    cargo install sqlx-cli --no-default-features --features postgres
fi

# Run migrations
sqlx migrate run
echo "✅ Database migrations completed"

echo ""
echo "🔨 Building backend..."
cargo build
echo "✅ Build completed"

echo ""
echo "🧪 Running tests..."
cargo test --lib
echo "✅ Tests passed"

echo ""
echo "======================================"
echo "✨ Setup Complete!"
echo ""
echo "Next steps:"
echo "  1. Edit config.toml and change all secrets"
echo "  2. Start backend: cargo run"
echo "  3. Backend will be available at http://localhost:8080"
echo "  4. Check health: curl http://localhost:8080/health"
echo ""
echo "For frontend setup:"
echo "  cd ../frontend"
echo "  npm install"
echo "  npm run dev"
echo ""
echo "For Raspberry Pi:"
echo "  cd ../../sensor_reader.py"
echo "  Edit send_to_backend.py with your backend IP"
echo "  python3 send_to_backend.py"
echo ""
echo "📚 Documentation:"
echo "  - README.md - Overview and quick start"
echo "  - IMPLEMENTATION_GUIDE.md - Detailed setup guide"
echo "  - ARCHITECTURE.md - System architecture"
echo ""
