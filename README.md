# 🏥 MedHealth Monitor - Advanced IoT Health Monitoring System

A production-grade health monitoring system built with Rust, featuring real-time sensor data processing, ML-based anomaly detection, and FHIR compliance.

## 📋 Table of Contents
- [Architecture](#architecture)
- [Features](#features)
- [Tech Stack](#tech-stack)
- [Prerequisites](#prerequisites)
- [Quick Start](#quick-start)
- [Configuration](#configuration)
- [API Documentation](#api-documentation)
- [Testing](#testing)
- [Deployment](#deployment)
- [Security](#security)
- [HIPAA Compliance](#hipaa-compliance)
- [Team Contribution Table](#Team-Contribution-Table)

## 🏗️ Architecture

```
Raspberry Pi (Sensor) → HMAC Auth → Rust Backend → PostgreSQL/Redis
                                          ↓
                                    Split Pipeline:
                                    - Real-time (SSE)
                                    - Storage (PostgreSQL)
                                    - ML Analysis
                                    - FHIR Conversion
                                          ↓
                               React Frontend (JWT Auth)
```

### Data Flow
1. **Sensor Layer**: MAX30102 sensor reads HR, SpO2, temperature
2. **Ingestion**: HMAC-authenticated POST to `/api/device/vitals`
3. **Processing Pipeline**:
   - Branch 1: Cache in Redis → Broadcast via SSE
   - Branch 2: Store in PostgreSQL → ML Analysis → FHIR Conversion
4. **Frontend**: SSE stream for real-time updates, JWT for authentication

## ✨ Features

### Backend (Rust)
- ✅ **JWT Authentication** with token revocation
- ✅ **HMAC Device Authentication** with replay protection
- ✅ **PostgreSQL** with SQLx migrations
- ✅ **Redis** caching for latest 100 readings
- ✅ **Server-Sent Events (SSE)** for real-time streaming
- ✅ **ML Anomaly Detection** (heart rate, SpO2, temperature)
- ✅ **FHIR R4 Compliance** (Observation resources with LOINC codes)
- ✅ **HIPAA-Compliant Logging** with audit trails
- ✅ **Property-Based Testing** with proptest
- ✅ **CI/CD Pipeline** with GitHub Actions
- ✅ **Prometheus Metrics** (planned)
- ✅ **Docker Compose** for local development

### Frontend (React + TypeScript)
- ✅ Real-time D3.js visualization
- ✅ EventSource SSE integration
- ✅ JWT token management
- ✅ ML alert notifications
- ✅ FHIR data export

## 🛠️ Tech Stack

### Backend
- **Rust** (Edition 2021)
- **actix-web** - Web framework
- **sqlx** - PostgreSQL driver
- **redis** - Caching
- **jsonwebtoken** - JWT authentication
- **argon2** - Password hashing
- **tracing** - Structured logging
- **ndarray + smartcore** - ML computations

### Frontend
- **React 18** + **TypeScript**
- **Vite** - Build tool
- **D3.js** - Data visualization
- **React Router** - Navigation

### Infrastructure
- **PostgreSQL 16** - Primary database
- **Redis 7** - Cache layer
- **Docker Compose** - Development environment

## 📦 Prerequisites

- **Rust** 1.75+ ([install](https://rustup.rs/))
- **Node.js** 18+ ([install](https://nodejs.org/))
- **Docker** & **Docker Compose** ([install](https://docs.docker.com/get-docker/))
- **PostgreSQL** 16+ (or use Docker)
- **Redis** 7+ (or use Docker)

### For Raspberry Pi
- Python 3.8+
- I2C enabled
- MAX30102 sensor connected

## 🚀 Quick Start

### 1. Clone the Repository
```bash
git clone <your-repo-url>
cd "D:\D disk\DIT HI\3 sem\Innovation and Complexity Management\Project"
```

### 2. Start Infrastructure (PostgreSQL + Redis)
```bash
cd website/backend
docker-compose up -d
```

### 3. Configure Backend
```bash
cp config.example.toml config.toml
# Edit config.toml with your settings
```

### 4. Run Database Migrations
```bash
cargo install sqlx-cli --no-default-features --features postgres
sqlx migrate run
```

### 5. Start Backend
```bash
cd website/backend
cargo run --release
```

Backend will start on `http://localhost:8080`

### 6. Start Frontend
```bash
cd website/frontend
npm install
npm run dev
```

Frontend will start on `http://localhost:5173`

### 7. Configure Raspberry Pi Client
```bash
cd sensor_reader.py
pip install -r requirements.txt

# Edit send_to_backend.py with your backend IP and secrets
python send_to_backend.py
```

## ⚙️ Configuration

### Backend Configuration (`config.toml`)

```toml
[server]
bind_addr = "0.0.0.0:8080"
workers = 4

[database]
url = "postgresql://user:pass@localhost:5432/medhealth_db"
max_connections = 20

[redis]
url = "redis://localhost:6379"
pool_size = 10

[jwt]
secret = "your-secret-key-min-32-chars"
expiration_hours = 24

[device]
secret = "your-device-secret"
replay_window_seconds = 60

[ml]
anomaly_threshold = 0.85
enable_alerts = true
critical_hr_low = 40
critical_hr_high = 180
critical_spo2_low = 88
```

### Frontend Configuration (`.env`)

```env
VITE_API_BASE_URL=http://localhost:8080
```

## 📡 API Documentation

### Authentication Endpoints

#### POST `/auth/signup`
Create a new user account.

**Request:**
```json
{
  "email": "user@example.com",
  "password": "securePassword123"
}
```

**Response:**
```json
{
  "token": "eyJ0eXAiOiJKV1QiLCJhbGc...",
  "refresh_token": "eyJ0eXAiOiJKV1QiLCJhbGc...",
  "user": {
    "id": "uuid",
    "email": "user@example.com",
    "role": "viewer"
  }
}
```

#### POST `/auth/login`
Login with existing credentials.

#### POST `/auth/logout`
Revoke current JWT token (requires Authorization header).

### Data Endpoints

#### GET `/api/vitals/latest`
Get the most recent vitals reading.

**Headers:** `Authorization: Bearer <token>`

**Response:**
```json
{
  "heartRate": 75,
  "spo2": 98,
  "temperature": 36.8,
  "timestamp": 1234567890,
  "quality_score": 0.95,
  "ml_alert": null
}
```

#### GET `/api/stream/vitals`
Server-Sent Events stream for real-time vitals.

**Headers:** `Authorization: Bearer <token>`

**Events:**
- `vitals` - New sensor reading
- `alert` - ML-generated alert
- `heartbeat` - Connection keepalive

#### POST `/api/device/vitals`
Device data ingestion (HMAC-protected).

**Headers:**
- `X-Device-Id: pi-001`
- `X-Timestamp: 1234567890`
- `X-Signature: <base64-hmac-sha256>`

**Request:**
```json
{
  "heartRate": 75,
  "spo2": 98,
  "temperature": 36.8,
  "timestamp": 1234567890
}
```

## 🧪 Testing

### Run All Tests
```bash
cd website/backend
cargo test
```

### Run Specific Test Suites
```bash
# Unit tests
cargo test --lib

# Integration tests
cargo test --test integration_test

# Property-based tests
cargo test --test property_tests
```

### Code Coverage
```bash
cargo install cargo-tarpaulin
cargo tarpaulin --out Html --output-dir coverage
```

## 🚢 Deployment

### Production Checklist
- [ ] Change all secrets in `config.toml`
- [ ] Enable HTTPS (TLS certificates)
- [ ] Configure CORS for production domain
- [ ] Set `cookie_secure(true)` in session middleware
- [ ] Enable PostgreSQL connection pooling
- [ ] Configure Redis persistence
- [ ] Set up log rotation
- [ ] Enable Prometheus metrics export
- [ ] Configure firewall rules
- [ ] Set up monitoring & alerting

### Docker Deployment
```bash
# Build production image
docker build -t medhealth-backend:latest .

# Run with environment variables
docker run -d \
  --name medhealth \
  -p 8080:8080 \
  -e DATABASE_URL=postgresql://... \
  -e REDIS_URL=redis://... \
  medhealth-backend:latest
```

## 🔒 Security

### Authentication Flow
1. User signs up/logs in → Receives JWT token
2. Token stored securely (httpOnly cookie or memory)
3. Every API request includes `Authorization: Bearer <token>`
4. Backend validates JWT, checks revocation table
5. On logout, token added to revocation table

### Device Authentication
1. Device sends request with headers:
   - `X-Device-Id`: Device identifier
   - `X-Timestamp`: Unix timestamp (replay protection)
   - `X-Signature`: HMAC-SHA256(`${timestamp}.${json_body}`)
2. Backend verifies signature with device secret
3. Backend checks timestamp is within 60s window

### HIPAA Compliance

#### Audit Logging
All data access is logged with:
- Event type (login, data_access, export)
- User ID / Device ID
- Action performed
- Timestamp
- IP address
- Success/failure status

#### Data Encryption
- Passwords: Argon2 hashing
- Tokens: JWT with HMAC-SHA256
- Transit: TLS 1.3 (in production)
- At rest: PostgreSQL encryption (optional)

#### PHI Protection
- NO PHI in log files
- Access control with role-based permissions
- Audit trail for all data access
- Automatic session timeout

## 📊 Evaluation Criteria Mapping

| Criterion | Implementation | Status |
|-----------|---------------|--------|
| **Development Environment** | Docker Compose, cross-platform, Cargo cache | ✅ |
| **Testing** | Unit, integration, property-based with proptest | ✅ |
| **Configuration** | TOML + env vars, SQLx migrations, CI/CD | ✅ |
| **Logging** | HIPAA-compliant with tracing, JSON format, audit trails | ✅ |
| **Architecture** | Async pipeline, PostgreSQL + Redis, SSE streaming | ✅ |
| **Input Validation** | validator crate, type-safe models, constraints | ✅ |
| **Error Handling** | thiserror, anyhow, proper HTTP status codes | ✅ |
| **Authentication** | JWT + HMAC, token revocation, Argon2 | ✅ |
| **Fault Tolerance** | Connection pooling, retry logic, health checks | ✅ |
| **FHIR Compliance** | FHIR R4 Observation, LOINC codes, JSONB storage | ✅ |



##  Team Contribution Table  
**Project: Smart-Walker – IoT Health Monitoring System**

| Team Member | Role | Key Technical Areas | Major Contributions | Core Files / Components |
|------------|------|---------------------|---------------------|--------------------------|
| **Nouman** | Backend Core & Sensor Application Layer Lead | System architecture, Rust backend, security, ML service, sensor threading | Designed system architecture; built Rust backend core; implemented JWT + HMAC authentication; developed ML anomaly detection; created HTTP handlers; built sensor application layer including threading, rolling buffers, finger detection, smoothing algorithms, and thread-safe integration | `main.rs`, `auth.rs`, `handlers.rs`, `ml_service.rs`, `middleware.rs`, `config.rs`, `heartrate_monitor.py`, `ARCHITECTURE.md` |
| **Zain** | Backend Services, Infrastructure & Hardware Integration Lead | Database, FHIR, SSE, Redis, logging, hardware integration, data pipeline | Designed DB schema and migrations; implemented FHIR R4 service; built SSE streaming; implemented Redis caching; created audit logging; set up Docker Compose; integrated sensors; built data pipeline; implemented real-time HR/SpO₂ computation and alerts; system testing | `database.rs`, `fhir_service.rs`, `sse.rs`, `redis_cache.rs`, `models.rs`, `logging.rs`, `docker-compose.yml`, `send_to_backend.py` |
| **Haris** | Frontend, Signal Processing & Documentation Lead | React frontend, visualization, signal processing | Built React dashboard with D3.js; implemented authentication UI and routing; developed API client; styled frontend; implemented HR & SpO₂ algorithms; integrated and validated algorithms; wrote documentation | `dashboard.tsx`, `auth.tsx`, `app.tsx`, `api.ts`, `style.css`, `hrcalc.py`, `README.md`, `QUICK_START.md` |
| **Hassan** | Hardware Driver & Testing Lead | Sensor driver, I2C communication, hardware validation | Developed MAX30102 driver; implemented FIFO buffer handling and 18-bit extraction; performed hardware validation using Linux I2C tools; wrote bash testing scripts; configured Raspberry Pi interfaces; handled hardware abstraction and power stability | `max30102.py`, bash scripts, Raspberry Pi configuration |


