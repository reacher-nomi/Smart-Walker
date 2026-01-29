# 🚀 Quick Start for Codespaces

## One Command Setup

Simply run from the root directory:

docker-compose up
That's it! The setup script will automatically:
- Generate secure secrets if not provided
- Create `.env` file with all required variables
- Start PostgreSQL, Redis, and pgAdmin
- Create backend configuration

## What Happens Automatically

When you create a Codespace:
1. The `.devcontainer/setup.sh` script runs automatically
2. All environment variables are generated
3. Docker services start automatically
4. Database migrations run
5. Dependencies are installed

## Manual Setup (if needed)

If you need to run setup manually:

# Make setup script executable
chmod +x setup.sh

# Run setup
./setup.sh

# Then start services
docker-compose up## Environment Variables

The setup automatically generates these if not provided via GitHub Secrets:
- `POSTGRES_PASSWORD` - Database password
- `PGADMIN_PASSWORD` - pgAdmin password
- `MEDHEALTH__JWT__SECRET` - JWT signing secret
- `MEDHEALTH__DEVICE__SECRET` - Device authentication secret

All secrets are stored in `.env` (gitignored).

## Running the Application

### Backend
cd website/backend
cargo run### Frontend
cd website/frontend
npm install
npm run dev## Services

- **PostgreSQL**: `localhost:5433`
- **Redis**: `localhost:6379`
- **pgAdmin**: `localhost:5050`
- **Backend API**: `localhost:8080`
- **Frontend**: `localhost:5173`

## Troubleshooting

### If `docker-compose up` fails:
1. Check if `.env` file exists: `ls -la .env`
2. If not, run: `./setup.sh`
3. Then: `docker-compose up`

### If services don't start:
docker-compose logs
docker-compose ps### Check environment variables:
cat .env
