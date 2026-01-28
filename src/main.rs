mod auth;
mod config;
mod database;
mod fhir_service;
mod handlers;
mod logging;
mod middleware;
mod ml_service;
mod models;
mod redis_cache;
mod sse;

use crate::config::Settings;
use crate::database::{create_pool, run_migrations};
use crate::handlers::AppState;
use crate::middleware::{AuditLogger, RequestId};
use actix_cors::Cors;
use actix_web::{http::header, middleware::Logger, web, App, HttpServer};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Load configuration
    let settings = Settings::new().expect("Failed to load configuration");

    // Initialize logging
    let log_dir = std::path::Path::new(&settings.logging.audit_log_path)
        .parent()
        .unwrap_or(std::path::Path::new("./logs"));
    
    logging::init_logging(log_dir, &settings.logging.level)
        .expect("Failed to initialize logging");

    info!("🚀 MedHealth Backend starting...");
    info!("Configuration loaded: {}", settings.server.bind_addr);

    // Create database pool
    info!("Connecting to PostgreSQL...");
    let pool = create_pool(&settings.database)
        .await
        .expect("Failed to create database pool");

    // Run migrations
    info!("Running database migrations...");
    run_migrations(&pool)
        .await
        .expect("Failed to run migrations");

    // Create Redis cache
    info!("Connecting to Redis...");
    let redis = redis_cache::RedisCache::new(&settings.redis)
        .await
        .expect("Failed to connect to Redis");
    let redis = Arc::new(RwLock::new(redis));

    // Initialize services
    let jwt_auth = Arc::new(auth::JwtAuth::new(&settings.jwt));
    let ml_service = Arc::new(ml_service::MlService::new(settings.ml.clone()));
    let fhir_service = Arc::new(fhir_service::FhirService::new(settings.fhir.clone()));
    let sse_broadcaster = sse::create_broadcaster();

    // Create app state
    let app_state = web::Data::new(AppState {
        pool: pool.clone(),
        redis: redis.clone(),
        jwt_auth: jwt_auth.clone(),
        ml_service: ml_service.clone(),
        fhir_service: fhir_service.clone(),
        sse_broadcaster: sse_broadcaster.clone(),
        device_secret: settings.device.secret.clone(),
    });

    info!("✅ All services initialized successfully");
    info!("🌐 Starting server on {}", settings.server.bind_addr);

    let bind_addr = settings.server.bind_addr.clone();
    let cors_origins = settings.cors.allowed_origins.clone();

    HttpServer::new(move || {
        // CORS configuration
        let mut cors = Cors::default()
            .allowed_methods(vec!["GET", "POST", "OPTIONS"])
            .allowed_headers(vec![
                header::CONTENT_TYPE,
                header::AUTHORIZATION,
                header::ACCEPT,
            ])
            .supports_credentials()
            .max_age(3600);

        for origin in &cors_origins {
            cors = cors.allowed_origin(origin);
        }

        App::new()
            // Middleware
            .wrap(Logger::default())
            .wrap(AuditLogger)
            .wrap(RequestId)
            .wrap(cors)
            // App state
            .app_data(app_state.clone())
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(sse_broadcaster.clone()))
            // Health check
            .route("/health", web::get().to(handlers::health_check))
            // Authentication routes
            .route("/auth/signup", web::post().to(handlers::signup))
            .route("/auth/login", web::post().to(handlers::login))
            .route("/auth/logout", web::post().to(handlers::logout))
            // API routes (JWT protected)
            .route("/api/vitals/latest", web::get().to(handlers::get_latest_vitals))
            // SSE stream (JWT protected in production)
            .route("/api/stream/vitals", web::get().to(sse::stream_vitals))
            // Device ingestion (HMAC protected)
            .route("/api/device/vitals", web::post().to(handlers::device_ingest))
    })
    .workers(settings.server.workers.unwrap_or(4))
    .bind(bind_addr)?
    .run()
    .await
}
