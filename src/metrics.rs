use prometheus::{
    Encoder, IntCounter, IntCounterVec, IntGauge, IntGaugeVec, Histogram, HistogramVec,
    Opts, Registry, TextEncoder,
};
use lazy_static::lazy_static;
use actix_web::{HttpResponse, Responder};

lazy_static! {
    pub static ref REGISTRY: Registry = Registry::new();

    // Request metrics
    pub static ref HTTP_REQUESTS_TOTAL: IntCounterVec = IntCounterVec::new(
        Opts::new("http_requests_total", "Total number of HTTP requests"),
        &["method", "endpoint", "status"]
    ).unwrap();

    pub static ref HTTP_REQUEST_DURATION: HistogramVec = HistogramVec::new(
        prometheus::HistogramOpts::new(
            "http_request_duration_seconds",
            "HTTP request duration in seconds"
        ),
        &["method", "endpoint"]
    ).unwrap();

    // Authentication metrics
    pub static ref AUTH_ATTEMPTS_TOTAL: IntCounterVec = IntCounterVec::new(
        Opts::new("auth_attempts_total", "Total authentication attempts"),
        &["result"] // "success" or "failure"
    ).unwrap();

    pub static ref ACTIVE_SESSIONS: IntGauge = IntGauge::new(
        "active_sessions",
        "Number of active user sessions"
    ).unwrap();

    // Device metrics
    pub static ref DEVICE_READINGS_TOTAL: IntCounterVec = IntCounterVec::new(
        Opts::new("device_readings_total", "Total sensor readings received"),
        &["device_id"]
    ).unwrap();

    pub static ref DEVICE_ERRORS_TOTAL: IntCounterVec = IntCounterVec::new(
        Opts::new("device_errors_total", "Total device errors"),
        &["device_id", "error_type"]
    ).unwrap();

    // ML metrics
    pub static ref ML_ANOMALIES_DETECTED: IntCounterVec = IntCounterVec::new(
        Opts::new("ml_anomalies_detected", "Total anomalies detected by ML"),
        &["alert_level"]
    ).unwrap();

    pub static ref ML_ANALYSIS_DURATION: Histogram = Histogram::with_opts(
        prometheus::HistogramOpts::new(
            "ml_analysis_duration_seconds",
            "ML analysis duration in seconds"
        )
    ).unwrap();

    // Database metrics
    pub static ref DB_CONNECTIONS_ACTIVE: IntGauge = IntGauge::new(
        "db_connections_active",
        "Number of active database connections"
    ).unwrap();

    pub static ref DB_QUERY_DURATION: HistogramVec = HistogramVec::new(
        prometheus::HistogramOpts::new(
            "db_query_duration_seconds",
            "Database query duration in seconds"
        ),
        &["query_type"]
    ).unwrap();

    // Cache metrics
    pub static ref CACHE_HITS: IntCounter = IntCounter::new(
        "cache_hits_total",
        "Total cache hits"
    ).unwrap();

    pub static ref CACHE_MISSES: IntCounter = IntCounter::new(
        "cache_misses_total",
        "Total cache misses"
    ).unwrap();

    // SSE metrics
    pub static ref SSE_CONNECTIONS_ACTIVE: IntGauge = IntGauge::new(
        "sse_connections_active",
        "Number of active SSE connections"
    ).unwrap();

    pub static ref SSE_EVENTS_SENT: IntCounterVec = IntCounterVec::new(
        Opts::new("sse_events_sent_total", "Total SSE events sent"),
        &["event_type"]
    ).unwrap();

    // Vitals metrics (anonymized for HIPAA compliance)
    pub static ref VITALS_HR_CURRENT: IntGauge = IntGauge::new(
        "vitals_heart_rate_current",
        "Current heart rate reading"
    ).unwrap();

    pub static ref VITALS_SPO2_CURRENT: IntGauge = IntGauge::new(
        "vitals_spo2_current",
        "Current SpO2 reading"
    ).unwrap();
}

/// Initialize Prometheus metrics
pub fn init_metrics() -> Result<(), prometheus::Error> {
    REGISTRY.register(Box::new(HTTP_REQUESTS_TOTAL.clone()))?;
    REGISTRY.register(Box::new(HTTP_REQUEST_DURATION.clone()))?;
    REGISTRY.register(Box::new(AUTH_ATTEMPTS_TOTAL.clone()))?;
    REGISTRY.register(Box::new(ACTIVE_SESSIONS.clone()))?;
    REGISTRY.register(Box::new(DEVICE_READINGS_TOTAL.clone()))?;
    REGISTRY.register(Box::new(DEVICE_ERRORS_TOTAL.clone()))?;
    REGISTRY.register(Box::new(ML_ANOMALIES_DETECTED.clone()))?;
    REGISTRY.register(Box::new(ML_ANALYSIS_DURATION.clone()))?;
    REGISTRY.register(Box::new(DB_CONNECTIONS_ACTIVE.clone()))?;
    REGISTRY.register(Box::new(DB_QUERY_DURATION.clone()))?;
    REGISTRY.register(Box::new(CACHE_HITS.clone()))?;
    REGISTRY.register(Box::new(CACHE_MISSES.clone()))?;
    REGISTRY.register(Box::new(SSE_CONNECTIONS_ACTIVE.clone()))?;
    REGISTRY.register(Box::new(SSE_EVENTS_SENT.clone()))?;
    REGISTRY.register(Box::new(VITALS_HR_CURRENT.clone()))?;
    REGISTRY.register(Box::new(VITALS_SPO2_CURRENT.clone()))?;
    
    Ok(())
}

/// Prometheus metrics endpoint handler
pub async fn metrics_handler() -> impl Responder {
    let encoder = TextEncoder::new();
    let metric_families = REGISTRY.gather();
    let mut buffer = Vec::new();
    
    if let Err(e) = encoder.encode(&metric_families, &mut buffer) {
        return HttpResponse::InternalServerError().body(format!("Failed to encode metrics: {}", e));
    }
    
    match String::from_utf8(buffer) {
        Ok(metrics) => HttpResponse::Ok()
            .content_type("text/plain; version=0.0.4")
            .body(metrics),
        Err(e) => HttpResponse::InternalServerError().body(format!("Failed to convert metrics: {}", e)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_initialization() {
        let result = init_metrics();
        assert!(result.is_ok());
    }

    #[test]
    fn test_http_requests_counter() {
        HTTP_REQUESTS_TOTAL
            .with_label_values(&["GET", "/api/vitals", "200"])
            .inc();
        
        let metric = HTTP_REQUESTS_TOTAL
            .with_label_values(&["GET", "/api/vitals", "200"])
            .get();
        
        assert!(metric >= 1);
    }
}
