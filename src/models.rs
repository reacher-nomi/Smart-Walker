use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use validator::Validate;

// ============ User Models ============

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub role: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_login_at: Option<DateTime<Utc>>,
    pub failed_login_attempts: i32,
    pub locked_until: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct SignupRequest {
    #[validate(email)]
    pub email: String,
    #[validate(length(min = 8))]
    pub password: String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct LoginRequest {
    #[validate(email)]
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub token: String,
    pub refresh_token: String,
    pub user: UserResponse,
}

#[derive(Debug, Serialize)]
pub struct UserResponse {
    pub id: Uuid,
    pub email: String,
    pub role: String,
}

// ============ Device Models ============

#[derive(Debug, Clone, FromRow)]
pub struct Device {
    pub id: Uuid,
    pub device_id: String,
    pub device_name: String,
    pub secret_hash: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub last_seen_at: Option<DateTime<Utc>>,
    pub metadata: serde_json::Value,
}

// ============ Sensor Reading Models ============

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct SensorReading {
    pub id: i64,
    pub device_id: Uuid,
    pub heart_rate: Option<i32>,
    pub spo2: Option<i32>,
    pub temperature: Option<f32>,
    pub reading_timestamp: DateTime<Utc>,
    pub received_at: DateTime<Utc>,
    pub quality_score: Option<f32>,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct DeviceVitalsIngest {
    #[validate(range(min = 0, max = 300))]
    pub heartRate: i32,
    #[validate(range(min = 0, max = 100))]
    pub spo2: i32,
    #[validate(range(min = 25.0, max = 45.0))]
    pub temperature: f32,
    pub timestamp: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LatestVitals {
    pub heartRate: i32,
    pub spo2: i32,
    pub temperature: f32,
    pub timestamp: i64,
    pub quality_score: Option<f32>,
    pub ml_alert: Option<String>,
}

// ============ ML Analysis Models ============

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct MlAnalysis {
    pub id: i64,
    pub sensor_reading_id: i64,
    pub anomaly_detected: bool,
    pub anomaly_score: Option<f32>,
    pub classification: Option<String>,
    pub alert_level: Option<String>,
    pub analysis_details: serde_json::Value,
    pub analyzed_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Clone)]
pub struct MlAlert {
    pub level: String,
    pub message: String,
    pub details: serde_json::Value,
}

// ============ FHIR Models ============

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct FhirObservation {
    pub id: Uuid,
    pub sensor_reading_id: i64,
    pub resource: serde_json::Value,
    pub resource_type: String,
    pub subject_reference: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct FhirObservationResource {
    pub resourceType: String,
    pub id: String,
    pub status: String,
    pub code: FhirCodeableConcept,
    pub subject: Option<FhirReference>,
    pub effectiveDateTime: String,
    pub valueQuantity: FhirQuantity,
    pub device: FhirReference,
}

#[derive(Debug, Serialize)]
pub struct FhirCodeableConcept {
    pub coding: Vec<FhirCoding>,
    pub text: String,
}

#[derive(Debug, Serialize)]
pub struct FhirCoding {
    pub system: String,
    pub code: String,
    pub display: String,
}

#[derive(Debug, Serialize)]
pub struct FhirReference {
    pub reference: String,
}

#[derive(Debug, Serialize)]
pub struct FhirQuantity {
    pub value: f32,
    pub unit: String,
    pub system: String,
    pub code: String,
}

// ============ Audit Log Models ============

#[derive(Debug, Clone, FromRow)]
pub struct AuditLog {
    pub id: i64,
    pub event_type: String,
    pub user_id: Option<Uuid>,
    pub device_id: Option<Uuid>,
    pub action: String,
    pub resource_type: Option<String>,
    pub resource_id: Option<String>,
    pub ip_address: Option<std::net::IpAddr>,
    pub user_agent: Option<String>,
    pub success: bool,
    pub error_message: Option<String>,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

// ============ JWT Claims ============

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,  // user email
    pub user_id: Uuid,
    pub role: String,
    pub exp: i64,     // expiration timestamp
    pub iat: i64,     // issued at
    pub jti: Uuid,    // JWT ID (for revocation)
}

// ============ SSE Event Models ============

#[derive(Debug, Serialize, Clone)]
#[serde(tag = "type")]
pub enum SseEvent {
    #[serde(rename = "vitals")]
    Vitals { data: LatestVitals },
    #[serde(rename = "alert")]
    Alert { data: MlAlert },
    #[serde(rename = "heartbeat")]
    Heartbeat { timestamp: i64 },
}
