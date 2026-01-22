# 🏗️ MedHealth Architecture Documentation

## System Overview

MedHealth is a distributed health monitoring system with three main tiers:
1. **Edge Layer** (Raspberry Pi + Sensors)
2. **Backend Layer** (Rust microservices)
3. **Presentation Layer** (React frontend)

---

## Complete Data Flow

```
┌─────────────────────────────────────────────────────────────────────┐
│                        EDGE LAYER                                    │
│  ┌──────────────┐    ┌─────────────┐    ┌────────────────────────┐ │
│  │  MAX30102    │───▶│ Python      │───▶│ HTTP Client (HMAC)     │ │
│  │  Sensor      │    │ Processing  │    │ POST /api/device/vitals│ │
│  │  - Heart Rate│    │ - hrcalc.py │    │ Headers:               │ │
│  │  - SpO2      │    │ - Quality   │    │   X-Device-Id          │ │
│  │  - Temp*     │    │   Check     │    │   X-Timestamp          │ │
│  └──────────────┘    └─────────────┘    │   X-Signature (HMAC)   │ │
│                                          └────────────────────────┘ │
└─────────────────────────────────────────────────────────────────────┘
                                  │
                                  ▼ (HTTPS in production)
┌─────────────────────────────────────────────────────────────────────┐
│                      BACKEND LAYER (Rust)                            │
│  ┌───────────────────────────────────────────────────────────────┐  │
│  │                  INGESTION LAYER                               │  │
│  │  ┌──────────────────────────────────────────────────────────┐ │  │
│  │  │ actix-web Handler: device_ingest()                       │ │  │
│  │  │ 1. Verify HMAC signature                                 │ │  │
│  │  │ 2. Check timestamp (±60s replay protection)              │ │  │
│  │  │ 3. Validate payload with validator crate                 │ │  │
│  │  └──────────────────────────────────────────────────────────┘ │  │
│  └───────────────────────────────────────────────────────────────┘  │
│                                  │                                   │
│                                  ▼                                   │
│  ┌───────────────────────────────────────────────────────────────┐  │
│  │              FIRST SPLIT: Async Processing Pipeline          │  │
│  │                                                               │  │
│  │  ┌────────────────────────┬─────────────────────────────────┐│  │
│  │  │   BRANCH 1: Real-Time  │   BRANCH 2: Persistence & ML    ││  │
│  │  │   (Zero Latency)       │   (Async Processing)            ││  │
│  │  └────────────────────────┴─────────────────────────────────┘│  │
│  └───────────────────────────────────────────────────────────────┘  │
│         │                                    │                       │
│         ▼                                    ▼                       │
│  ┌─────────────────────┐         ┌──────────────────────────────┐  │
│  │   REDIS CACHE       │         │   POSTGRESQL (SQLx)          │  │
│  │   ├─ Latest vitals  │         │   ├─ sensor_readings         │  │
│  │   ├─ Last 100 reads │         │   ├─ ml_analysis             │  │
│  │   └─ TTL: 1 hour    │         │   ├─ fhir_observations       │  │
│  └─────────────────────┘         │   ├─ users                   │  │
│         │                         │   ├─ devices                 │  │
│         ▼                         │   └─ audit_logs              │  │
│  ┌─────────────────────┐         └──────────────────────────────┘  │
│  │   SSE BROADCASTER   │                      │                     │
│  │   (tokio::broadcast)│                      ▼                     │
│  │   ├─ vitals events  │         ┌──────────────────────────────┐  │
│  │   ├─ alert events   │         │   ML SERVICE                 │  │
│  │   └─ heartbeat      │         │   ├─ Anomaly Detection       │  │
│  └─────────────────────┘         │   ├─ Quality Assessment      │  │
│         │                         │   ├─ Statistical Analysis    │  │
│         │                         │   └─ Alert Generation        │  │
│         │                         └──────────────────────────────┘  │
│         │                                      │                     │
│         │                                      ▼                     │
│         │                         ┌──────────────────────────────┐  │
│         │                         │   FHIR SERVICE               │  │
│         │                         │   ├─ LOINC 8867-4 (HR)       │  │
│         │                         │   ├─ LOINC 2708-6 (SpO2)     │  │
│         │                         │   └─ LOINC 8310-5 (Temp)     │  │
│         │                         └──────────────────────────────┘  │
│         │                                                             │
│  ┌──────┴─────────────────────────────────────────────────────────┐ │
│  │                   AUTHENTICATION & AUTHORIZATION                │ │
│  │  ┌──────────────────────┐  ┌────────────────────────────────┐ │ │
│  │  │ JWT Auth             │  │ HMAC Device Auth               │ │ │
│  │  │ ├─ jsonwebtoken      │  │ ├─ HMAC-SHA256                 │ │ │
│  │  │ ├─ Token revocation  │  │ ├─ Timestamp validation        │ │ │
│  │  │ └─ Role-based access │  │ └─ Device secret lookup        │ │ │
│  │  └──────────────────────┘  └────────────────────────────────┘ │ │
│  └─────────────────────────────────────────────────────────────────┘ │
│                                                                      │
│  ┌──────────────────────────────────────────────────────────────┐   │
│  │                  MIDDLEWARE STACK                             │   │
│  │  ├─ Request ID (UUID)                                        │   │
│  │  ├─ CORS (configurable origins)                              │   │
│  │  ├─ Audit Logger (HIPAA-compliant)                           │   │
│  │  ├─ Metrics Collector (Prometheus)                           │   │
│  │  └─ Error Handler (thiserror)                                │   │
│  └──────────────────────────────────────────────────────────────┘   │
│                                                                      │
│  ┌──────────────────────────────────────────────────────────────┐   │
│  │                  LOGGING & MONITORING                         │   │
│  │  ├─ tracing (structured logs)                                │   │
│  │  ├─ JSON format for SIEM integration                         │   │
│  │  ├─ Daily log rotation                                       │   │
│  │  ├─ NO PHI in logs (HIPAA)                                   │   │
│  │  └─ Prometheus metrics export                                │   │
│  └──────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────┘
                                  │
                                  ▼ (REST + SSE)
┌─────────────────────────────────────────────────────────────────────┐
│                    PRESENTATION LAYER (React)                        │
│  ┌──────────────────────────────────────────────────────────────┐   │
│  │  Auth Pages                                                  │   │
│  │  ├─ Login (JWT token acquisition)                           │   │
│  │  └─ Signup (Argon2 password hashing on backend)             │   │
│  └──────────────────────────────────────────────────────────────┘   │
│  ┌──────────────────────────────────────────────────────────────┐   │
│  │  Dashboard (Real-Time)                                       │   │
│  │  ├─ EventSource connection to SSE endpoint                  │   │
│  │  ├─ D3.js line chart (heart rate over time)                 │   │
│  │  ├─ Vitals cards (HR, SpO2, Temp)                           │   │
│  │  ├─ ML alert notifications                                  │   │
│  │  └─ Quality score indicator                                 │   │
│  └──────────────────────────────────────────────────────────────┘   │
│  ┌──────────────────────────────────────────────────────────────┐   │
│  │  Data Export (Future)                                        │   │
│  │  └─ FHIR Bundle download                                     │   │
│  └──────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────┘
```

---

## Security Architecture

### 1. Device Authentication (HMAC-SHA256)

```
Device Request:
  POST /api/device/vitals
  Headers:
    X-Device-Id: "pi-001"
    X-Timestamp: "1234567890"
    X-Signature: "base64(HMAC-SHA256(secret, timestamp.json))"
  Body: {"heartRate": 75, "spo2": 98, "temperature": 36.8, "timestamp": 1234567890}

Backend Verification:
  1. Lookup device by device_id
  2. Reconstruct message: "${timestamp}.${json_body}"
  3. Compute: HMAC-SHA256(device_secret, message)
  4. Compare signatures (constant-time)
  5. Verify |now - timestamp| < 60 seconds (replay protection)
```

### 2. User Authentication (JWT)

```
Login Flow:
  POST /auth/login → {"email": ..., "password": ...}
  ├─ Verify password with Argon2
  ├─ Generate JWT with claims:
  │    {
  │      "sub": "user@example.com",
  │      "user_id": "uuid",
  │      "role": "viewer",
  │      "exp": timestamp + 24h,
  │      "jti": "uuid" (for revocation)
  │    }
  └─ Return token

Protected Request:
  GET /api/vitals/latest
  Headers: Authorization: Bearer eyJ0eXAi...
  ├─ Extract token
  ├─ Verify signature with JWT secret
  ├─ Check expiration
  ├─ Check revocation table
  └─ Allow/Deny based on role
```

### 3. HIPAA Compliance Measures

- **Audit Logging**: All data access logged to `audit_logs` table
- **No PHI in Logs**: Only user IDs and device IDs (no actual vitals in log files)
- **Encryption at Rest**: PostgreSQL encryption (optional)
- **Encryption in Transit**: TLS 1.3 (production)
- **Access Control**: Role-based (admin, viewer, clinician)
- **Session Management**: JWT with short expiration, revocation support
- **Data Minimization**: Only collect necessary fields

---

## Database Schema

### Core Tables

```sql
users
├─ id (UUID, PK)
├─ email (unique)
├─ password_hash (Argon2)
├─ role (admin|viewer|clinician)
├─ is_active
├─ failed_login_attempts
└─ locked_until

devices
├─ id (UUID, PK)
├─ device_id (unique, e.g., "pi-001")
├─ secret_hash
├─ is_active
└─ last_seen_at

sensor_readings
├─ id (BIGSERIAL, PK)
├─ device_id (FK → devices)
├─ heart_rate (0-300)
├─ spo2 (0-100)
├─ temperature (25-45°C)
├─ reading_timestamp
├─ quality_score (0-1)
└─ metadata (JSONB)

ml_analysis
├─ id (BIGSERIAL, PK)
├─ sensor_reading_id (FK)
├─ anomaly_detected
├─ anomaly_score
├─ classification (normal|warning|critical|artifact)
├─ alert_level (none|low|medium|high|critical)
└─ analysis_details (JSONB)

fhir_observations
├─ id (UUID, PK)
├─ sensor_reading_id (FK)
├─ resource (JSONB - full FHIR Observation)
└─ subject_reference (Patient/123)

audit_logs
├─ id (BIGSERIAL, PK)
├─ event_type
├─ user_id
├─ action
├─ ip_address
├─ success
└─ created_at

revoked_tokens
├─ jti (UUID, PK)
├─ user_id
├─ revoked_at
└─ expires_at
```

---

## ML Service Algorithm

### Anomaly Detection Pipeline

```rust
fn analyze_reading(reading: SensorReading) -> MlAnalysisResult {
    1. Critical Threshold Checks
       - HR < 40 or HR > 180 → Bradycardia/Tachycardia
       - SpO2 < 88 → Hypoxemia
       - Temp > 38.0 → Fever
       - Temp < 35.5 → Hypothermia

    2. Statistical Anomaly Detection
       - Calculate Z-score: (value - mean) / std_dev
       - If |Z| > 3 → Statistical anomaly

    3. Signal Quality Assessment
       - Check for zero values (no signal)
       - Check for unrealistic values
       - Score: 0.0 - 1.0

    4. Classification
       - anomaly_score = weighted sum of issues
       - if score < 0.5 → "normal"
       - if score < 0.8 → "warning"
       - else → "critical"

    5. Alert Generation
       - if score >= threshold (0.85) → Generate alert
       - Alert levels: low|medium|high|critical

    6. Future: Time-Series Analysis
       - Sliding window detection
       - Trend analysis
       - LSTM for predictive alerts
}
```

---

## FHIR Implementation

### LOINC Codes Used

- **8867-4**: Heart rate
- **2708-6**: Oxygen saturation in Arterial blood
- **8310-5**: Body temperature

### Example FHIR Observation

```json
{
  "resourceType": "Observation",
  "id": "uuid",
  "status": "final",
  "code": {
    "coding": [{
      "system": "http://loinc.org",
      "code": "8867-4",
      "display": "Heart rate"
    }],
    "text": "Heart Rate"
  },
  "subject": {
    "reference": "Patient/123"
  },
  "effectiveDateTime": "2026-01-22T10:30:00Z",
  "valueQuantity": {
    "value": 75,
    "unit": "beats/minute",
    "system": "http://unitsofmeasure.org",
    "code": "/min"
  },
  "device": {
    "reference": "Device/pi-001"
  }
}
```

---

## Performance Characteristics

### Latency Targets
- Device ingestion → Redis cache: **<10ms**
- SSE event broadcast: **<50ms**
- ML analysis: **<100ms**
- FHIR conversion: **<50ms**
- PostgreSQL write: **<200ms**

### Scalability
- **Concurrent devices**: 100+ with single instance
- **Concurrent SSE clients**: 1000+ with single instance
- **Throughput**: 100+ readings/second
- **Database**: Partitioning by timestamp for large datasets

### Resource Usage
- **Memory**: ~200MB baseline + ~1MB per SSE connection
- **CPU**: 2 cores recommended, scales with traffic
- **Database**: Connection pool of 20 (configurable)
- **Redis**: 256MB max memory with LRU eviction

---

## Deployment Architecture (Production)

```
Internet
  │
  ▼
┌─────────────────┐
│  Load Balancer  │ (nginx/HAProxy)
│  - TLS          │
│  - Rate limiting│
└─────────────────┘
  │
  ├──────────────┬──────────────┐
  ▼              ▼              ▼
┌────────┐   ┌────────┐   ┌────────┐
│Backend │   │Backend │   │Backend │ (Multiple instances)
│Instance│   │Instance│   │Instance│
└────────┘   └────────┘   └────────┘
  │              │              │
  └──────────────┴──────────────┘
                 │
         ┌───────┴───────┐
         ▼               ▼
   ┌──────────┐   ┌──────────┐
   │PostgreSQL│   │  Redis   │ (Primary + Replica)
   │ Primary  │   │ Sentinel │
   │ + Replica│   └──────────┘
   └──────────┘
```

---

## Monitoring & Observability

### Prometheus Metrics
- `http_requests_total` - Request counter
- `http_request_duration_seconds` - Request latency
- `auth_attempts_total` - Authentication attempts
- `device_readings_total` - Sensor readings received
- `ml_anomalies_detected` - Anomalies by alert level
- `sse_connections_active` - Active SSE connections
- `db_connections_active` - Database pool usage
- `cache_hits_total` / `cache_misses_total` - Cache performance

### Health Checks
- `/health` - Overall system health
- `/metrics` - Prometheus metrics endpoint

### Logging
- **Format**: JSON (structured)
- **Rotation**: Daily
- **Retention**: 90 days (configurable)
- **Levels**: ERROR, WARN, INFO, DEBUG, TRACE

---

## Technology Choices Rationale

| Technology | Reason |
|------------|--------|
| **Rust** | Memory safety, performance, zero-cost abstractions |
| **actix-web** | High performance async web framework |
| **SQLx** | Compile-time checked SQL queries |
| **PostgreSQL** | ACID compliance, JSONB support for FHIR |
| **Redis** | Sub-millisecond latency for real-time cache |
| **SSE** | Simple, HTTP-based real-time streaming |
| **JWT** | Stateless authentication, scalable |
| **HMAC** | Symmetric crypto for device auth |
| **Argon2** | Memory-hard password hashing (resist GPU attacks) |
| **tracing** | Best-in-class structured logging for Rust |

---

For implementation details, see [IMPLEMENTATION_GUIDE.md](./IMPLEMENTATION_GUIDE.md).
