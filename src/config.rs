use config::{Config, ConfigError, File};
use serde::{Deserialize, Deserializer};

fn deserialize_allowed_origins<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum StringOrVec {
        String(String),
        Vec(Vec<String>),
    }
    match StringOrVec::deserialize(deserializer) {
        Ok(StringOrVec::String(s)) => Ok(s.split(',').map(|x| x.trim().to_string()).filter(|x| !x.is_empty()).collect()),
        Ok(StringOrVec::Vec(v)) => Ok(v),
        Err(e) => Err(e),
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Settings {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub redis: RedisConfig,
    pub jwt: JwtConfig,
    pub cors: CorsConfig,
    pub device: DeviceConfig,
    pub ml: MlConfig,
    pub fhir: FhirConfig,
    pub logging: LoggingConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub bind_addr: String,
    pub workers: Option<usize>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub min_connections: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RedisConfig {
    pub url: String,
    pub pool_size: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct JwtConfig {
    pub secret: String,
    pub expiration_hours: i64,
    #[allow(dead_code)] // Used by JwtAuth::generate_refresh_token for long-lived tokens
    pub refresh_token_days: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CorsConfig {
    #[serde(deserialize_with = "deserialize_allowed_origins")]
    pub allowed_origins: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DeviceConfig {
    pub secret: String,
    pub replay_window_seconds: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MlConfig {
    pub anomaly_threshold: f32,
    pub enable_alerts: bool,
    pub critical_hr_low: i32,
    pub critical_hr_high: i32,
    pub critical_spo2_low: i32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FhirConfig {
    pub base_url: String,
    pub organization_id: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
    pub audit_log_path: String,
    pub enable_phi_encryption: bool,
}

impl Settings {
    pub fn new() -> Result<Self, ConfigError> {
        let config = Config::builder()
            .add_source(File::with_name("config.toml").required(false))
            .add_source(config::Environment::with_prefix("MEDHEALTH").separator("__"))
            .build()?;

        config.try_deserialize()
    }
}
