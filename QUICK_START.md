# ⚡ Quick Start Guide

Get MedHealth running in 5 minutes!

## Windows Quick Start

### 1. Prerequisites
```powershell
# Install Rust
winget install Rustlang.Rustup

# Install Docker Desktop
winget install Docker.DockerDesktop

# Restart your terminal
```

### 2. Start Services
```powershell
cd "website\backend"
docker-compose up -d
```

### 3. Configure
```powershell
# Copy and edit config
copy config.example.toml config.toml
# Edit config.toml in your favorite editor
```

### 4. Run Migrations
```powershell
cargo install sqlx-cli --no-default-features --features postgres
sqlx migrate run
```

### 5. Start Backend
```powershell
cargo run
```

✅ Backend running at `http://localhost:8080`

### 6. Test It
```powershell
# Open PowerShell and test
curl http://localhost:8080/health
```

---

## Linux/Mac Quick Start

```bash
# 1. Install Rust (if needed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 2. Navigate to project
cd "website/backend"

# 3. Run setup script
chmod +x ../../setup.sh
../../setup.sh

# 4. Start backend
cargo run
```

---

## First API Calls

### Create User
```bash
curl -X POST http://localhost:8080/auth/signup \
  -H "Content-Type: application/json" \
  -d '{"email":"test@example.com","password":"password123"}'
```

Save the returned token!

### Get Latest Vitals (with token)
```bash
curl http://localhost:8080/api/vitals/latest \
  -H "Authorization: Bearer YOUR_TOKEN_HERE"
```

---

## Frontend Setup

```bash
cd website/frontend
npm install
npm run dev
```

Open `http://localhost:5173` in your browser.

---

## Raspberry Pi Client

```bash
cd sensor_reader.py

# Edit configuration
nano send_to_backend.py
# Update: BACKEND_URL, DEVICE_ID, DEVICE_SECRET

# Run client
python3 send_to_backend.py
```

---

## Troubleshooting

### "Failed to connect to database"
```bash
# Check if services are running
docker-compose ps

# Restart services
docker-compose restart
```

### "Port 8080 already in use"
Edit `config.toml`:
```toml
[server]
bind_addr = "0.0.0.0:8081"  # Change port
```

### "Cargo build fails"
```bash
# Update Rust
rustup update

# Clean and rebuild
cargo clean
cargo build
```

---

## Next Steps

1. ✅ Backend running
2. ✅ Frontend displaying dashboard
3. ✅ Raspberry Pi sending data
4. 📚 Read [IMPLEMENTATION_GUIDE.md](./IMPLEMENTATION_GUIDE.md) for details
5. 🏗️ See [ARCHITECTURE.md](./ARCHITECTURE.md) for system design
6. 🧪 Run tests: `cargo test`

---

## Production Checklist

Before deploying to production:

- [ ] Change ALL secrets in `config.toml`
- [ ] Set up HTTPS/TLS
- [ ] Configure CORS for your domain
- [ ] Enable database backups
- [ ] Set up monitoring
- [ ] Review HIPAA compliance checklist
- [ ] Run security audit: `cargo audit`
- [ ] Load test the system

---

**Need Help?** Check the full [README.md](./README.md) or [IMPLEMENTATION_GUIDE.md](./IMPLEMENTATION_GUIDE.md)
