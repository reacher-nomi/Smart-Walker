use tracing::subscriber::set_global_default;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{fmt, layer::SubscriberExt, EnvFilter, Registry};
use std::path::Path;

/// Initialize HIPAA-compliant logging
pub fn init_logging(log_dir: impl AsRef<Path>, log_level: &str) -> anyhow::Result<()> {
    // Create log directory if it doesn't exist
    std::fs::create_dir_all(&log_dir)?;

    // File appender for audit logs (daily rotation)
    let file_appender = RollingFileAppender::new(Rotation::DAILY, log_dir.as_ref(), "audit.log");

    // JSON formatter for structured logs (easier for SIEM integration)
    let file_layer = fmt::layer()
        .json()
        .with_writer(file_appender)
        .with_target(true)
        .with_thread_ids(true)
        .with_thread_names(true);

    // Console output for development
    let console_layer = fmt::layer()
        .with_target(true)
        .with_thread_ids(true);

    // Environment filter
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(log_level));

    // Combine layers
    let subscriber = Registry::default()
        .with(env_filter)
        .with(file_layer)
        .with(console_layer);

    set_global_default(subscriber)?;

    tracing::info!("Logging initialized with level: {}", log_level);

    Ok(())
}

/// Audit log macro for HIPAA compliance
/// DO NOT log PHI (Protected Health Information) directly
#[macro_export]
macro_rules! audit_log {
    ($event_type:expr, $action:expr, $user_id:expr, $success:expr) => {
        tracing::info!(
            event_type = $event_type,
            action = $action,
            user_id = ?$user_id,
            success = $success,
            timestamp = chrono::Utc::now().to_rfc3339(),
            "AUDIT_EVENT"
        );
    };
    ($event_type:expr, $action:expr, $user_id:expr, $success:expr, $metadata:expr) => {
        tracing::info!(
            event_type = $event_type,
            action = $action,
            user_id = ?$user_id,
            success = $success,
            metadata = ?$metadata,
            timestamp = chrono::Utc::now().to_rfc3339(),
            "AUDIT_EVENT"
        );
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_logging_initialization() {
        let temp_dir = tempdir().unwrap();
        let result = init_logging(temp_dir.path(), "info");
        assert!(result.is_ok());
    }
}
