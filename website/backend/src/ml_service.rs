use crate::config::MlConfig;
use crate::models::{MlAlert, SensorReading};
// ML computations (currently unused but available for future expansion)
use serde_json::json;

pub struct MlService {
    config: MlConfig,
}

impl MlService {
    pub fn new(config: MlConfig) -> Self {
        Self { config }
    }

    /// Analyze sensor reading for anomalies
    pub fn analyze_reading(&self, reading: &SensorReading) -> MlAnalysisResult {
        let mut anomalies = Vec::new();
        let mut anomaly_score = 0.0;
        let mut alert_level = "none".to_string();

        // Extract values with defaults for Option types
        let hr = reading.heart_rate.unwrap_or(0);
        let spo2 = reading.spo2.unwrap_or(0);
        let temp = reading.temperature.unwrap_or(0.0);

        // 1. Critical threshold checks
        if hr > 0 {
            if hr < self.config.critical_hr_low {
                anomalies.push("Bradycardia detected (low heart rate)");
                anomaly_score += 0.8;
                alert_level = "critical".to_string();
            } else if hr > self.config.critical_hr_high {
                anomalies.push("Tachycardia detected (high heart rate)");
                anomaly_score += 0.8;
                alert_level = "critical".to_string();
            }
        }

        if spo2 > 0 && spo2 < self.config.critical_spo2_low {
            anomalies.push("Hypoxemia detected (low SpO2)");
            anomaly_score += 0.9;
            alert_level = "critical".to_string();
        }

        // 2. Temperature anomalies
        if temp > 0.0 {
            if temp > 38.0 {
                anomalies.push("Fever detected");
                anomaly_score += 0.6;
                if alert_level == "none" {
                    alert_level = "high".to_string();
                }
            } else if temp < 35.5 {
                anomalies.push("Hypothermia risk");
                anomaly_score += 0.7;
                if alert_level == "none" {
                    alert_level = "high".to_string();
                }
            }
        }

        // 3. Signal quality assessment
        let quality_score = self.assess_signal_quality(hr, spo2, temp);
        
        if quality_score < 0.5 {
            anomalies.push("Poor signal quality detected");
            if alert_level == "none" {
                alert_level = "low".to_string();
            }
        }

        // 4. Statistical anomaly detection (simplified z-score)
        let hr_zscore = self.calculate_zscore(hr as f32, 70.0, 12.0);
        let spo2_zscore = self.calculate_zscore(spo2 as f32, 97.0, 2.0);
        
        if hr_zscore.abs() > 3.0 {
            anomalies.push("Statistical HR anomaly");
            anomaly_score += 0.5;
        }

        if spo2_zscore.abs() > 3.0 {
            anomalies.push("Statistical SpO2 anomaly");
            anomaly_score += 0.5;
        }

        // 5. Classification
        let classification = if anomaly_score == 0.0 {
            "normal"
        } else if anomaly_score < 0.5 {
            "warning"
        } else if anomaly_score < 0.8 {
            "critical"
        } else {
            "critical"
        };

        // Normalize anomaly score to 0-1
        let final_score = (anomaly_score / 2.0_f32).min(1.0);

        MlAnalysisResult {
            anomaly_detected: !anomalies.is_empty(),
            anomaly_score: final_score,
            classification: classification.to_string(),
            alert_level,
            quality_score,
            details: json!({
                "anomalies": anomalies,
                "hr_zscore": hr_zscore,
                "spo2_zscore": spo2_zscore,
            }),
        }
    }

    /// Assess signal quality based on reading values
    fn assess_signal_quality(&self, hr: i32, spo2: i32, temp: f32) -> f32 {
        let mut quality: f32 = 1.0;

        // Penalize if values are zero (no signal)
        if hr == 0 {
            quality -= 0.4;
        }
        if spo2 == 0 {
            quality -= 0.4;
        }
        if temp == 0.0 {
            quality -= 0.2;
        }

        // Penalize unrealistic values
        if hr > 250 || spo2 > 100 || temp > 43.0 || temp < 30.0 {
            quality -= 0.3;
        }

        quality.max(0.0)
    }

    /// Calculate z-score for anomaly detection
    fn calculate_zscore(&self, value: f32, mean: f32, std_dev: f32) -> f32 {
        if std_dev == 0.0 {
            return 0.0;
        }
        (value - mean) / std_dev
    }

    /// Generate alert message if needed
    pub fn generate_alert(&self, analysis: &MlAnalysisResult) -> Option<MlAlert> {
        if !self.config.enable_alerts || !analysis.anomaly_detected {
            return None;
        }

        if analysis.anomaly_score < self.config.anomaly_threshold {
            return None;
        }

        let message = match analysis.alert_level.as_str() {
            "critical" => "Critical vital signs detected! Immediate attention required.".to_string(),
            "high" => "Abnormal vital signs detected. Medical review recommended.".to_string(),
            "medium" => "Unusual vital signs pattern detected.".to_string(),
            "low" => "Minor data quality issues detected.".to_string(),
            _ => return None,
        };

        Some(MlAlert {
            level: analysis.alert_level.clone(),
            message,
            details: analysis.details.clone(),
        })
    }

    /// Advanced: Time-series anomaly detection (placeholder for future implementation)
    pub fn detect_temporal_anomalies(&self, _readings: &[SensorReading]) -> Vec<String> {
        // TODO: Implement sliding window analysis, trend detection, etc.
        // This could use LSTM, isolation forest, or other ML models
        vec![]
    }
}

#[derive(Debug, Clone)]
pub struct MlAnalysisResult {
    pub anomaly_detected: bool,
    pub anomaly_score: f32,
    pub classification: String,
    pub alert_level: String,
    pub quality_score: f32,
    pub details: serde_json::Value,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use uuid::Uuid;

    fn create_test_config() -> MlConfig {
        MlConfig {
            anomaly_threshold: 0.85,
            enable_alerts: true,
            critical_hr_low: 40,
            critical_hr_high: 180,
            critical_spo2_low: 88,
        }
    }

    fn create_test_reading(hr: i32, spo2: i32, temp: f32) -> SensorReading {
        SensorReading {
            id: 1,
            device_id: Uuid::new_v4(),
            heart_rate: Some(hr),
            spo2: Some(spo2),
            temperature: Some(temp),
            reading_timestamp: Utc::now(),
            received_at: Utc::now(),
            quality_score: Some(1.0),
            metadata: json!({}),
        }
    }

    #[test]
    fn test_normal_reading() {
        let service = MlService::new(create_test_config());
        let reading = create_test_reading(72, 98, 36.8);
        
        let result = service.analyze_reading(&reading);
        
        assert_eq!(result.classification, "normal");
        assert!(!result.anomaly_detected);
    }

    #[test]
    fn test_tachycardia_detection() {
        let service = MlService::new(create_test_config());
        let reading = create_test_reading(195, 98, 36.8);
        
        let result = service.analyze_reading(&reading);
        
        assert!(result.anomaly_detected);
        assert_eq!(result.alert_level, "critical");
    }

    #[test]
    fn test_hypoxemia_detection() {
        let service = MlService::new(create_test_config());
        let reading = create_test_reading(75, 85, 36.8);
        
        let result = service.analyze_reading(&reading);
        
        assert!(result.anomaly_detected);
        assert_eq!(result.alert_level, "critical");
    }

    #[test]
    fn test_fever_detection() {
        let service = MlService::new(create_test_config());
        let reading = create_test_reading(75, 98, 38.5);
        
        let result = service.analyze_reading(&reading);
        
        assert!(result.anomaly_detected);
        assert_eq!(result.alert_level, "high");
    }

    #[test]
    fn test_signal_quality() {
        let service = MlService::new(create_test_config());
        let reading = create_test_reading(0, 0, 0.0);
        
        let result = service.analyze_reading(&reading);
        
        assert!(result.quality_score < 0.5);
    }
}
