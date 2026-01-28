use crate::auth::{extract_bearer_token, JwtAuth};
use crate::fhir_service::FhirService;
use crate::ml_service::MlService;
use crate::models::*;
use crate::redis_cache::RedisCache;
use crate::sse::{broadcast_alert, broadcast_vitals, SseBroadcaster};
use actix_web::{web, HttpRequest, HttpResponse, Responder};
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use argon2::password_hash::SaltString;
use chrono::Utc;
use hmac::{Hmac, Mac};
use rand::rngs::OsRng;
use sha2::Sha256;
use sqlx::PgPool;
use std::sync::Arc;
use tokio::sync::RwLock;
use validator::Validate;
use base64::{engine::general_purpose, Engine as _};

type HmacSha256 = Hmac<Sha256>;

pub struct AppState {
    pub pool: PgPool,
    pub redis: Arc<RwLock<RedisCache>>,
    pub jwt_auth: Arc<JwtAuth>,
    pub ml_service: Arc<MlService>,
    pub fhir_service: Arc<FhirService>,
    pub sse_broadcaster: SseBroadcaster,
    pub device_secret: String,
}

// ============ Health Check ============

pub async fn health_check(pool: web::Data<PgPool>) -> impl Responder {
    // Check database connection
    let db_ok = sqlx::query("SELECT 1").fetch_one(pool.get_ref()).await.is_ok();

    if db_ok {
        HttpResponse::Ok().json(serde_json::json!({
            "status": "healthy",
            "database": "connected",
            "timestamp": Utc::now().to_rfc3339()
        }))
    } else {
        HttpResponse::ServiceUnavailable().json(serde_json::json!({
            "status": "unhealthy",
            "database": "disconnected"
        }))
    }
}

// ============ Authentication Handlers ============

pub async fn signup(
    state: web::Data<AppState>,
    body: web::Json<SignupRequest>,
) -> impl Responder {
    // Validate input
    if let Err(e) = body.validate() {
        return HttpResponse::BadRequest().json(serde_json::json!({"error": e.to_string()}));
    }

    let email = body.email.trim().to_lowercase();

    // Check if user already exists
    let existing: Result<Option<User>, _> = sqlx::query_as("SELECT * FROM users WHERE email = $1")
        .bind(&email)
        .fetch_optional(&state.pool)
        .await;

    if existing.is_ok() && existing.unwrap().is_some() {
        return HttpResponse::Conflict().json(serde_json::json!({"error": "User already exists"}));
    }

    // Hash password
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let password_hash = match argon2.hash_password(body.password.as_bytes(), &salt) {
        Ok(hash) => hash.to_string(),
        Err(_) => return HttpResponse::InternalServerError().json(serde_json::json!({"error": "Password hashing failed"})),
    };

    // Insert user
    let user: Result<User, _> = sqlx::query_as(
        "INSERT INTO users (email, password_hash, role) VALUES ($1, $2, 'viewer') RETURNING *"
    )
    .bind(&email)
    .bind(&password_hash)
    .fetch_one(&state.pool)
    .await;

    match user {
        Ok(u) => {
            // Generate JWT tokens
            let token = state.jwt_auth.generate_token(u.id, &u.email, &u.role).unwrap();
            let refresh_token = state.jwt_auth.generate_token(u.id, &u.email, &u.role).unwrap(); // In production, use different exp

            HttpResponse::Ok().json(AuthResponse {
                token,
                refresh_token,
                user: UserResponse {
                    id: u.id,
                    email: u.email,
                    role: u.role,
                },
            })
        }
        Err(e) => {
            HttpResponse::InternalServerError().json(serde_json::json!({"error": format!("Database error: {}", e)}))
        }
    }
}

pub async fn login(
    state: web::Data<AppState>,
    body: web::Json<LoginRequest>,
) -> impl Responder {
    if let Err(e) = body.validate() {
        return HttpResponse::BadRequest().json(serde_json::json!({"error": e.to_string()}));
    }

    let email = body.email.trim().to_lowercase();

    // Find user
    let user: Result<User, _> = sqlx::query_as("SELECT * FROM users WHERE email = $1 AND is_active = true")
        .bind(&email)
        .fetch_one(&state.pool)
        .await;

    let user = match user {
        Ok(u) => u,
        Err(_) => return HttpResponse::Unauthorized().json(serde_json::json!({"error": "Invalid credentials"})),
    };

    // Check if account is locked
    if let Some(locked_until) = user.locked_until {
        if locked_until > Utc::now() {
            return HttpResponse::Forbidden().json(serde_json::json!({"error": "Account temporarily locked"}));
        }
    }

    // Verify password
    let parsed_hash = match PasswordHash::new(&user.password_hash) {
        Ok(h) => h,
        Err(_) => return HttpResponse::InternalServerError().json(serde_json::json!({"error": "Invalid password hash"})),
    };

    let argon2 = Argon2::default();
    if argon2.verify_password(body.password.as_bytes(), &parsed_hash).is_err() {
        // Increment failed login attempts
        let _ = sqlx::query("UPDATE users SET failed_login_attempts = failed_login_attempts + 1 WHERE id = $1")
            .bind(user.id)
            .execute(&state.pool)
            .await;

        return HttpResponse::Unauthorized().json(serde_json::json!({"error": "Invalid credentials"}));
    }

    // Reset failed attempts and update last login
    let _ = sqlx::query("UPDATE users SET failed_login_attempts = 0, last_login_at = now() WHERE id = $1")
        .bind(user.id)
        .execute(&state.pool)
        .await;

    // Generate JWT
    let token = state.jwt_auth.generate_token(user.id, &user.email, &user.role).unwrap();
    let refresh_token = state.jwt_auth.generate_token(user.id, &user.email, &user.role).unwrap();

    HttpResponse::Ok().json(AuthResponse {
        token,
        refresh_token,
        user: UserResponse {
            id: user.id,
            email: user.email,
            role: user.role,
        },
    })
}

pub async fn logout(
    req: HttpRequest,
    state: web::Data<AppState>,
) -> impl Responder {
    let auth_header = req.headers().get("Authorization").and_then(|h| h.to_str().ok());
    
    let token = match extract_bearer_token(auth_header) {
        Ok(t) => t,
        Err(_) => return HttpResponse::Unauthorized().json(serde_json::json!({"error": "Missing token"})),
    };

    let claims = match state.jwt_auth.validate_token(&token) {
        Ok(c) => c,
        Err(_) => return HttpResponse::Unauthorized().json(serde_json::json!({"error": "Invalid token"})),
    };

    // Revoke token
    let _ = state.jwt_auth.revoke_token(&claims, &state.pool).await;

    HttpResponse::Ok().json(serde_json::json!({"status": "logged_out"}))
}

// ============ Device Ingestion Handler ============

pub async fn device_ingest(
    req: HttpRequest,
    state: web::Data<AppState>,
    body: web::Json<DeviceVitalsIngest>,
) -> impl Responder {
    // Validate input
    if let Err(e) = body.validate() {
        return HttpResponse::BadRequest().json(serde_json::json!({"error": e.to_string()}));
    }

    // Extract HMAC headers
    let device_id = match req.headers().get("x-device-id").and_then(|h| h.to_str().ok()) {
        Some(id) => id,
        None => return HttpResponse::Unauthorized().json(serde_json::json!({"error": "Missing X-Device-Id"})),
    };

    let timestamp = match req.headers().get("x-timestamp").and_then(|h| h.to_str().ok()).and_then(|s| s.parse::<i64>().ok()) {
        Some(ts) => ts,
        None => return HttpResponse::Unauthorized().json(serde_json::json!({"error": "Missing X-Timestamp"})),
    };

    let signature = match req.headers().get("x-signature").and_then(|h| h.to_str().ok()) {
        Some(sig) => sig,
        None => return HttpResponse::Unauthorized().json(serde_json::json!({"error": "Missing X-Signature"})),
    };

    // Verify timestamp (replay protection)
    let now = Utc::now().timestamp();
    if (now - timestamp).abs() > 60 {
        return HttpResponse::Unauthorized().json(serde_json::json!({"error": "Timestamp out of range"}));
    }

    // Verify HMAC signature
    let json_body = serde_json::to_string(&*body).unwrap();
    let msg = format!("{}.{}", timestamp, json_body);
    let mut mac = HmacSha256::new_from_slice(state.device_secret.as_bytes()).unwrap();
    mac.update(msg.as_bytes());
    let expected_sig = general_purpose::STANDARD.encode(mac.finalize().into_bytes());

    if expected_sig != signature {
        return HttpResponse::Unauthorized().json(serde_json::json!({"error": "Invalid signature"}));
    }

    // Find device in database
    let device: Result<Device, _> = sqlx::query_as("SELECT * FROM devices WHERE device_id = $1 AND is_active = true")
        .bind(device_id)
        .fetch_one(&state.pool)
        .await;

    let device = match device {
        Ok(d) => d,
        Err(_) => return HttpResponse::Unauthorized().json(serde_json::json!({"error": "Unknown device"})),
    };

    // Create sensor reading
    let reading: Result<SensorReading, _> = sqlx::query_as(
        "INSERT INTO sensor_readings (device_id, heart_rate, spo2, temperature, reading_timestamp) 
         VALUES ($1, $2, $3, $4, to_timestamp($5)) RETURNING *"
    )
    .bind(device.id)
    .bind(body.heartRate)
    .bind(body.spo2)
    .bind(body.temperature)
    .bind(body.timestamp)
    .fetch_one(&state.pool)
    .await;

    let reading = match reading {
        Ok(r) => r,
        Err(e) => return HttpResponse::InternalServerError().json(serde_json::json!({"error": format!("Database error: {}", e)})),
    };

    // Run ML analysis
    let ml_result = state.ml_service.analyze_reading(&reading);
    
    // Store ML analysis
    let _ = sqlx::query(
        "INSERT INTO ml_analysis (sensor_reading_id, anomaly_detected, anomaly_score, classification, alert_level, analysis_details) 
         VALUES ($1, $2, $3, $4, $5, $6)"
    )
    .bind(reading.id)
    .bind(ml_result.anomaly_detected)
    .bind(ml_result.anomaly_score)
    .bind(&ml_result.classification)
    .bind(&ml_result.alert_level)
    .bind(&ml_result.details)
    .execute(&state.pool)
    .await;

    // Create FHIR observation
    let fhir_bundle = state.fhir_service.create_observation_bundle(&reading, None);
    
    let _ = sqlx::query(
        "INSERT INTO fhir_observations (sensor_reading_id, resource) VALUES ($1, $2)"
    )
    .bind(reading.id)
    .bind(&fhir_bundle)
    .execute(&state.pool)
    .await;

    // Prepare vitals for caching and broadcasting
    let vitals = LatestVitals {
        heartRate: body.heartRate,
        spo2: body.spo2,
        temperature: body.temperature,
        timestamp: body.timestamp,
        quality_score: Some(ml_result.quality_score),
        ml_alert: state.ml_service.generate_alert(&ml_result).map(|a| a.level),
    };

    // Cache in Redis
    let mut redis = state.redis.write().await;
    let _ = redis.set_latest_vitals(&vitals).await;
    drop(redis);

    // Broadcast via SSE
    broadcast_vitals(&state.sse_broadcaster, vitals.clone());

    // Broadcast alert if needed
    if let Some(alert) = state.ml_service.generate_alert(&ml_result) {
        broadcast_alert(&state.sse_broadcaster, alert);
    }

    HttpResponse::Ok().json(serde_json::json!({"status": "accepted", "reading_id": reading.id}))
}

// ============ Vitals Retrieval (JWT Protected) ============

pub async fn get_latest_vitals(
    req: HttpRequest,
    state: web::Data<AppState>,
) -> impl Responder {
    // Verify JWT
    let auth_header = req.headers().get("Authorization").and_then(|h| h.to_str().ok());
    let token = match extract_bearer_token(auth_header) {
        Ok(t) => t,
        Err(_) => return HttpResponse::Unauthorized().json(serde_json::json!({"error": "Missing token"})),
    };

    let claims = match state.jwt_auth.validate_token(&token) {
        Ok(c) => c,
        Err(_) => return HttpResponse::Unauthorized().json(serde_json::json!({"error": "Invalid token"})),
    };

    // Check if token is revoked
    if state.jwt_auth.is_token_revoked(claims.jti, &state.pool).await.unwrap_or(false) {
        return HttpResponse::Unauthorized().json(serde_json::json!({"error": "Token revoked"}));
    }

    // Try Redis first
    let mut redis = state.redis.write().await;
    if let Ok(Some(vitals)) = redis.get_latest_vitals().await {
        drop(redis);
        return HttpResponse::Ok().json(vitals);
    }
    drop(redis);

    // Fallback to database
    let reading: Result<SensorReading, _> = sqlx::query_as(
        "SELECT * FROM sensor_readings ORDER BY reading_timestamp DESC LIMIT 1"
    )
    .fetch_one(&state.pool)
    .await;

    match reading {
        Ok(r) => {
            let vitals = LatestVitals {
                heartRate: r.heart_rate.unwrap_or(0),
                spo2: r.spo2.unwrap_or(0),
                temperature: r.temperature.unwrap_or(0.0),
                timestamp: r.reading_timestamp.timestamp(),
                quality_score: r.quality_score,
                ml_alert: None,
            };
            HttpResponse::Ok().json(vitals)
        }
        Err(_) => HttpResponse::Ok().json(LatestVitals {
            heartRate: 0,
            spo2: 0,
            temperature: 0.0,
            timestamp: 0,
            quality_score: None,
            ml_alert: None,
        }),
    }
}

// ============ FHIR Export Handler ============

pub async fn export_fhir_bundle(
    req: HttpRequest,
    state: web::Data<AppState>,
    query: web::Query<serde_json::Value>,
) -> impl Responder {
    // Verify JWT
    let auth_header = req.headers().get("Authorization").and_then(|h| h.to_str().ok());
    let token = match extract_bearer_token(auth_header) {
        Ok(t) => t,
        Err(_) => return HttpResponse::Unauthorized().json(serde_json::json!({"error": "Missing token"})),
    };

    let claims = match state.jwt_auth.validate_token(&token) {
        Ok(c) => c,
        Err(_) => return HttpResponse::Unauthorized().json(serde_json::json!({"error": "Invalid token"})),
    };

    // Check if token is revoked
    if state.jwt_auth.is_token_revoked(claims.jti, &state.pool).await.unwrap_or(false) {
        return HttpResponse::Unauthorized().json(serde_json::json!({"error": "Token revoked"}));
    }

    // Get limit from query (default 100)
    let limit = query.get("limit")
        .and_then(|v| v.as_u64())
        .unwrap_or(100)
        .min(1000) as i64; // Max 1000 records

    // Fetch recent sensor readings
    let readings: Result<Vec<SensorReading>, _> = sqlx::query_as(
        "SELECT * FROM sensor_readings 
         ORDER BY reading_timestamp DESC 
         LIMIT $1"
    )
    .bind(limit)
    .fetch_all(&state.pool)
    .await;

    match readings {
        Ok(rs) => {
            // Convert each reading to FHIR observations
            let mut entries = Vec::new();
            
            for reading in &rs {
                let bundle = state.fhir_service.create_observation_bundle(reading, None);
                
                // Extract entries from bundle
                if let Some(entry_array) = bundle.get("entry").and_then(|e| e.as_array()) {
                    for entry in entry_array {
                        entries.push(entry.clone());
                    }
                }
            }

            // Create FHIR Bundle
            let fhir_bundle = serde_json::json!({
                "resourceType": "Bundle",
                "id": uuid::Uuid::new_v4().to_string(),
                "type": "collection",
                "timestamp": Utc::now().to_rfc3339(),
                "total": entries.len(),
                "entry": entries
            });

            HttpResponse::Ok()
                .content_type("application/fhir+json")
                .json(fhir_bundle)
        }
        Err(e) => {
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Failed to fetch readings: {}", e)
            }))
        }
    }
}
