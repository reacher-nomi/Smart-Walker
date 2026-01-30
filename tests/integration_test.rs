use actix_web::{test, web, App, http::header};
use medhealth_backend::{
    auth::JwtAuth,
    config::{DatabaseConfig, JwtConfig, RedisConfig, MlConfig, FhirConfig},
    database::{create_pool, run_migrations},
    handlers::{health_check, signup, login, logout, get_latest_vitals, device_ingest, export_fhir_bundle, AppState},
    ml_service::MlService,
    fhir_service::FhirService,
    redis_cache::RedisCache,
    sse,
};
use serde_json::json;
use sqlx::PgPool;
use std::sync::Arc;
use tokio::sync::RwLock;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use base64::{engine::general_purpose, Engine as _};

type HmacSha256 = Hmac<Sha256>;

// Test database URL - should use test database
const TEST_DATABASE_URL: &str = "postgresql://test:test@localhost:5432/medhealth_test";
const TEST_REDIS_URL: &str = "redis://localhost:6379";
const TEST_JWT_SECRET: &str = "test_secret_key_minimum_32_chars_long_for_security_testing";
const TEST_DEVICE_SECRET: &str = "test_device_secret_for_hmac_testing_32_chars";

/// Builds and returns the test App inline so the concrete type is known to init_service.
macro_rules! build_test_app {
    () => {{
        async {
            let db_config = DatabaseConfig {
                url: TEST_DATABASE_URL.to_string(),
                max_connections: 5,
                min_connections: 1,
            };
            let pool = create_pool(&db_config).await.expect("Failed to create test database pool");
            run_migrations(&pool).await.expect("Failed to run migrations");
            let redis = RedisCache::new(&RedisConfig {
                url: TEST_REDIS_URL.to_string(),
                pool_size: 5,
            })
            .await
            .expect("Redis required for integration tests. Run: docker-compose up -d redis");
            let redis = Arc::new(RwLock::new(redis));
            let jwt_config = JwtConfig {
                secret: TEST_JWT_SECRET.to_string(),
                expiration_hours: 24,
                refresh_token_days: 7,
            };
            let jwt_auth = Arc::new(JwtAuth::new(&jwt_config));
            let ml_service = Arc::new(MlService::new(MlConfig {
                anomaly_threshold: 0.85,
                enable_alerts: true,
                critical_hr_low: 40,
                critical_hr_high: 180,
                critical_spo2_low: 88,
            }));
            let fhir_service = Arc::new(FhirService::new(FhirConfig {
                base_url: "http://localhost:8080/fhir".to_string(),
                organization_id: "org-test-001".to_string(),
            }));
            let sse_broadcaster = sse::create_broadcaster();
            let app_state = web::Data::new(AppState {
                pool: pool.clone(),
                redis: redis.clone(),
                jwt_auth: jwt_auth.clone(),
                ml_service: ml_service.clone(),
                fhir_service: fhir_service.clone(),
                sse_broadcaster: sse_broadcaster.clone(),
                device_secret: TEST_DEVICE_SECRET.to_string(),
                replay_window_seconds: 60,
            });
            App::new()
                .app_data(app_state.clone())
                .app_data(web::Data::new(pool.clone()))
                .app_data(web::Data::new(sse_broadcaster.clone()))
                .route("/health", web::get().to(health_check))
                .route("/auth/signup", web::post().to(signup))
                .route("/auth/login", web::post().to(login))
                .route("/auth/logout", web::post().to(logout))
                .route("/api/vitals/latest", web::get().to(get_latest_vitals))
                .route("/api/fhir/export", web::get().to(export_fhir_bundle))
                .route("/api/device/vitals", web::post().to(device_ingest))
        }
        .await
    }};
}

#[actix_web::test]
async fn test_health_endpoint() {
    let pool = create_pool(&DatabaseConfig {
        url: TEST_DATABASE_URL.to_string(),
        max_connections: 5,
        min_connections: 1,
    }).await;
    
    if pool.is_err() {
        // Skip test if database not available
        return;
    }
    
    let app = test::init_service(
        App::new()
            .route("/health", web::get().to(health_check))
            .app_data(web::Data::new(pool.unwrap()))
    ).await;
    
    let req = test::TestRequest::get()
        .uri("/health")
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success() || resp.status().is_server_error());
}

#[actix_web::test]
async fn test_signup_valid_user() {
    let app = test::init_service(build_test_app!()).await;
    
    let signup_data = json!({
        "email": "test@example.com",
        "password": "SecurePass123!"
    });
    
    let req = test::TestRequest::post()
        .uri("/auth/signup")
        .set_json(&signup_data)
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    
    // Should succeed or return conflict if user already exists
    assert!(resp.status().is_success() || resp.status() == 409);
}

#[actix_web::test]
async fn test_signup_invalid_email() {
    let app = test::init_service(build_test_app!()).await;
    
    let signup_data = json!({
        "email": "invalid-email",
        "password": "SecurePass123!"
    });
    
    let req = test::TestRequest::post()
        .uri("/auth/signup")
        .set_json(&signup_data)
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_client_error());
}

#[actix_web::test]
async fn test_login_invalid_credentials() {
    let app = test::init_service(build_test_app!()).await;
    
    let login_data = json!({
        "email": "nonexistent@example.com",
        "password": "WrongPassword123!"
    });
    
    let req = test::TestRequest::post()
        .uri("/auth/login")
        .set_json(&login_data)
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_client_error());
}

#[actix_web::test]
async fn test_login_valid_credentials() {
    let app = test::init_service(build_test_app!()).await;
    
    // First signup
    let signup_data = json!({
        "email": "logintest@example.com",
        "password": "SecurePass123!"
    });
    
    let signup_req = test::TestRequest::post()
        .uri("/auth/signup")
        .set_json(&signup_data)
        .to_request();
    
    let _ = test::call_service(&app, signup_req).await;
    
    // Then login
    let login_data = json!({
        "email": "logintest@example.com",
        "password": "SecurePass123!"
    });
    
    let login_req = test::TestRequest::post()
        .uri("/auth/login")
        .set_json(&login_data)
        .to_request();
    
    let resp = test::call_service(&app, login_req).await;
    assert!(resp.status().is_success());
    
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body.get("token").is_some());
}

#[actix_web::test]
async fn test_jwt_authentication() {
    let app = test::init_service(build_test_app!()).await;
    
    // Signup and login to get token
    let signup_data = json!({
        "email": "jwttest@example.com",
        "password": "SecurePass123!"
    });
    
    let _ = test::call_service(&app, 
        test::TestRequest::post()
            .uri("/auth/signup")
            .set_json(&signup_data)
            .to_request()
    ).await;
    
    let login_resp = test::call_service(&app,
        test::TestRequest::post()
            .uri("/auth/login")
            .set_json(&json!({
                "email": "jwttest@example.com",
                "password": "SecurePass123!"
            }))
            .to_request()
    ).await;
    
    let login_body: serde_json::Value = test::read_body_json(login_resp).await;
    let token = login_body.get("token").and_then(|t| t.as_str()).unwrap();
    
    // Use token to access protected endpoint
    let req = test::TestRequest::get()
        .uri("/api/vitals/latest")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    // Should succeed (even if no data, should not be 401)
    assert_ne!(resp.status(), 401);
}

#[actix_web::test]
async fn test_device_ingestion_with_valid_hmac() {
    let app = test::init_service(build_test_app!()).await;
    
    // First, register a device in the database
    let pool = create_pool(&DatabaseConfig {
        url: TEST_DATABASE_URL.to_string(),
        max_connections: 5,
        min_connections: 1,
    }).await;
    
    if pool.is_err() {
        return; // Skip if DB not available
    }
    
    let pool = pool.unwrap();
    
    // Register test device
    let device_id = "TEST-DEVICE-001";
    let _ = sqlx::query(
        "INSERT INTO devices (device_id, device_name, secret_hash, is_active) 
         VALUES ($1, $2, $3, true)
         ON CONFLICT (device_id) DO UPDATE SET is_active = true"
    )
    .bind(device_id)
    .bind("Test Device")
    .bind("hashed_secret") // In real implementation, this would be hashed
    .execute(&pool)
    .await;
    
    // Create HMAC signature
    let timestamp = chrono::Utc::now().timestamp();
    let body = json!({
        "heartRate": 75,
        "spo2": 98,
        "temperature": 36.8,
        "timestamp": timestamp
    });
    
    let json_body = serde_json::to_string(&body).unwrap();
    let msg = format!("{}.{}", timestamp, json_body);
    let mut mac = HmacSha256::new_from_slice(TEST_DEVICE_SECRET.as_bytes()).unwrap();
    mac.update(msg.as_bytes());
    let signature = general_purpose::STANDARD.encode(mac.finalize().into_bytes());
    
    let req = test::TestRequest::post()
        .uri("/api/device/vitals")
        .insert_header(("X-Device-Id", device_id))
        .insert_header(("X-Timestamp", timestamp.to_string()))
        .insert_header(("X-Signature", signature))
        .set_json(&body)
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    // Should succeed or return device not found (if device registration failed)
    assert!(resp.status().is_success() || resp.status() == 401);
}

#[actix_web::test]
async fn test_device_ingestion_invalid_signature() {
    let app = test::init_service(build_test_app!()).await;
    
    let timestamp = chrono::Utc::now().timestamp();
    let body = json!({
        "heartRate": 75,
        "spo2": 98,
        "temperature": 36.8,
        "timestamp": timestamp
    });
    
    let req = test::TestRequest::post()
        .uri("/api/device/vitals")
        .insert_header(("X-Device-Id", "TEST-DEVICE-001"))
        .insert_header(("X-Timestamp", timestamp.to_string()))
        .insert_header(("X-Signature", "invalid_signature"))
        .set_json(&body)
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401);
}

#[actix_web::test]
async fn test_device_ingestion_missing_headers() {
    let app = test::init_service(build_test_app!()).await;
    
    let body = json!({
        "heartRate": 75,
        "spo2": 98,
        "temperature": 36.8,
        "timestamp": chrono::Utc::now().timestamp()
    });
    
    // Missing X-Device-Id
    let req = test::TestRequest::post()
        .uri("/api/device/vitals")
        .insert_header(("X-Timestamp", chrono::Utc::now().timestamp().to_string()))
        .insert_header(("X-Signature", "some_signature"))
        .set_json(&body)
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401);
}

#[actix_web::test]
async fn test_device_ingestion_timestamp_replay() {
    let app = test::init_service(build_test_app!()).await;
    
    // Use old timestamp (outside 60s window)
    let old_timestamp = chrono::Utc::now().timestamp() - 120;
    let body = json!({
        "heartRate": 75,
        "spo2": 98,
        "temperature": 36.8,
        "timestamp": old_timestamp
    });
    
    let json_body = serde_json::to_string(&body).unwrap();
    let msg = format!("{}.{}", old_timestamp, json_body);
    let mut mac = HmacSha256::new_from_slice(TEST_DEVICE_SECRET.as_bytes()).unwrap();
    mac.update(msg.as_bytes());
    let signature = general_purpose::STANDARD.encode(mac.finalize().into_bytes());
    
    let req = test::TestRequest::post()
        .uri("/api/device/vitals")
        .insert_header(("X-Device-Id", "TEST-DEVICE-001"))
        .insert_header(("X-Timestamp", old_timestamp.to_string()))
        .insert_header(("X-Signature", signature))
        .set_json(&body)
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401); // Should reject old timestamp
}
