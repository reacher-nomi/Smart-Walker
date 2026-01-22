use crate::config::RedisConfig;
use crate::models::LatestVitals;
use redis::{aio::ConnectionManager, AsyncCommands, RedisError};
use serde_json;

const LATEST_VITALS_KEY: &str = "vitals:latest";
const RECENT_READINGS_KEY: &str = "readings:recent";
const MAX_RECENT_READINGS: isize = 100;

pub struct RedisCache {
    client: ConnectionManager,
}

impl RedisCache {
    pub async fn new(config: &RedisConfig) -> Result<Self, RedisError> {
        let client = redis::Client::open(config.url.as_str())?;
        let conn = ConnectionManager::new(client).await?;
        
        Ok(Self { client: conn })
    }

    /// Store the latest vitals reading
    pub async fn set_latest_vitals(&mut self, vitals: &LatestVitals) -> Result<(), RedisError> {
        let json = serde_json::to_string(vitals)
            .map_err(|e| RedisError::from((redis::ErrorKind::TypeError, "Serialization failed", e.to_string())))?;
        
        self.client.set::<_, _, ()>(LATEST_VITALS_KEY, json).await?;
        
        // Also add to recent readings list (LPUSH + LTRIM for max 100)
        let reading_json = serde_json::to_string(&vitals)
            .map_err(|e| RedisError::from((redis::ErrorKind::TypeError, "Serialization failed", e.to_string())))?;
        
        self.client.lpush::<_, _, ()>(RECENT_READINGS_KEY, reading_json).await?;
        self.client.ltrim::<_, ()>(RECENT_READINGS_KEY, 0, MAX_RECENT_READINGS - 1).await?;
        
        Ok(())
    }

    /// Get the latest vitals reading
    pub async fn get_latest_vitals(&mut self) -> Result<Option<LatestVitals>, RedisError> {
        let json: Option<String> = self.client.get(LATEST_VITALS_KEY).await?;
        
        match json {
            Some(data) => {
                let vitals = serde_json::from_str(&data)
                    .map_err(|e| RedisError::from((redis::ErrorKind::TypeError, "Deserialization failed", e.to_string())))?;
                Ok(Some(vitals))
            }
            None => Ok(None),
        }
    }

    /// Get recent readings (last N readings)
    pub async fn get_recent_readings(&mut self, count: isize) -> Result<Vec<LatestVitals>, RedisError> {
        let json_list: Vec<String> = self.client.lrange(RECENT_READINGS_KEY, 0, count - 1).await?;
        
        let mut readings = Vec::new();
        for json in json_list {
            if let Ok(vitals) = serde_json::from_str(&json) {
                readings.push(vitals);
            }
        }
        
        Ok(readings)
    }

    /// Check if Redis is healthy
    pub async fn health_check(&mut self) -> Result<bool, RedisError> {
        let _: String = redis::cmd("PING").query_async(&mut self.client).await?;
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_redis_latest_vitals() {
        // This requires a running Redis instance
        // Skip in CI unless Redis is available
        if std::env::var("CI").is_ok() {
            return;
        }

        let config = RedisConfig {
            url: "redis://localhost:6379".to_string(),
            pool_size: 5,
        };

        let mut cache = RedisCache::new(&config).await.expect("Redis connection failed");

        let vitals = LatestVitals {
            heartRate: 75,
            spo2: 98,
            temperature: 36.5,
            timestamp: 1234567890,
            quality_score: Some(0.95),
            ml_alert: None,
        };

        cache.set_latest_vitals(&vitals).await.expect("Failed to set vitals");
        
        let retrieved = cache.get_latest_vitals().await.expect("Failed to get vitals");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().heartRate, 75);
    }
}
