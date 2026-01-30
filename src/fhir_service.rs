use crate::config::FhirConfig;
use crate::models::{
    FhirCodeableConcept, FhirCoding, FhirObservationResource, FhirQuantity, FhirReference,
    SensorReading,
};
use chrono::Utc;
use serde_json::{json, Value};
use uuid::Uuid;

pub struct FhirService {
    config: FhirConfig,
}

impl FhirService {
    pub fn new(config: FhirConfig) -> Self {
        Self { config }
    }

    /// Convert a sensor reading to FHIR Observation resource for Heart Rate
    pub fn create_heart_rate_observation(
        &self,
        reading: &SensorReading,
        patient_reference: Option<String>,
    ) -> Value {
        let heart_rate = reading.heart_rate.unwrap_or(0) as f32;

        let observation = FhirObservationResource {
            resourceType: "Observation".to_string(),
            id: Uuid::new_v4().to_string(),
            status: "final".to_string(),
            code: FhirCodeableConcept {
                coding: vec![FhirCoding {
                    system: "http://loinc.org".to_string(),
                    code: "8867-4".to_string(),
                    display: "Heart rate".to_string(),
                }],
                text: "Heart Rate".to_string(),
            },
            subject: patient_reference.map(|r| FhirReference { reference: r }),
            effectiveDateTime: reading.reading_timestamp.to_rfc3339(),
            valueQuantity: FhirQuantity {
                value: heart_rate,
                unit: "beats/minute".to_string(),
                system: "http://unitsofmeasure.org".to_string(),
                code: "/min".to_string(),
            },
            device: FhirReference {
                reference: format!("Device/{}", reading.device_id),
            },
        };

        serde_json::to_value(observation).unwrap_or(json!({}))
    }

    /// Convert a sensor reading to FHIR Observation resource for SpO2
    pub fn create_spo2_observation(
        &self,
        reading: &SensorReading,
        patient_reference: Option<String>,
    ) -> Value {
        let spo2 = reading.spo2.unwrap_or(0) as f32;

        let observation = FhirObservationResource {
            resourceType: "Observation".to_string(),
            id: Uuid::new_v4().to_string(),
            status: "final".to_string(),
            code: FhirCodeableConcept {
                coding: vec![FhirCoding {
                    system: "http://loinc.org".to_string(),
                    code: "2708-6".to_string(),
                    display: "Oxygen saturation in Arterial blood".to_string(),
                }],
                text: "Oxygen Saturation (SpO2)".to_string(),
            },
            subject: patient_reference.map(|r| FhirReference { reference: r }),
            effectiveDateTime: reading.reading_timestamp.to_rfc3339(),
            valueQuantity: FhirQuantity {
                value: spo2,
                unit: "percent".to_string(),
                system: "http://unitsofmeasure.org".to_string(),
                code: "%".to_string(),
            },
            device: FhirReference {
                reference: format!("Device/{}", reading.device_id),
            },
        };

        serde_json::to_value(observation).unwrap_or(json!({}))
    }

    /// Convert a sensor reading to FHIR Observation resource for Body Temperature
    pub fn create_temperature_observation(
        &self,
        reading: &SensorReading,
        patient_reference: Option<String>,
    ) -> Value {
        let temp = reading.temperature.unwrap_or(0.0);

        let observation = FhirObservationResource {
            resourceType: "Observation".to_string(),
            id: Uuid::new_v4().to_string(),
            status: "final".to_string(),
            code: FhirCodeableConcept {
                coding: vec![FhirCoding {
                    system: "http://loinc.org".to_string(),
                    code: "8310-5".to_string(),
                    display: "Body temperature".to_string(),
                }],
                text: "Body Temperature".to_string(),
            },
            subject: patient_reference.map(|r| FhirReference { reference: r }),
            effectiveDateTime: reading.reading_timestamp.to_rfc3339(),
            valueQuantity: FhirQuantity {
                value: temp,
                unit: "degrees Celsius".to_string(),
                system: "http://unitsofmeasure.org".to_string(),
                code: "Cel".to_string(),
            },
            device: FhirReference {
                reference: format!("Device/{}", reading.device_id),
            },
        };

        serde_json::to_value(observation).unwrap_or(json!({}))
    }

    /// Create a FHIR Bundle containing all observations for a reading
    pub fn create_observation_bundle(
        &self,
        reading: &SensorReading,
        patient_reference: Option<String>,
    ) -> Value {
        let mut entries = vec![];

        if reading.heart_rate.is_some() {
            entries.push(json!({
                "resource": self.create_heart_rate_observation(reading, patient_reference.clone())
            }));
        }

        if reading.spo2.is_some() {
            entries.push(json!({
                "resource": self.create_spo2_observation(reading, patient_reference.clone())
            }));
        }

        if reading.temperature.is_some() {
            entries.push(json!({
                "resource": self.create_temperature_observation(reading, patient_reference.clone())
            }));
        }

        json!({
            "resourceType": "Bundle",
            "id": Uuid::new_v4().to_string(),
            "type": "collection",
            "timestamp": Utc::now().to_rfc3339(),
            "meta": {
                "source": self.config.base_url,
                "tag": [{
                    "system": "http://medhealth.local/organization",
                    "code": self.config.organization_id
                }]
            },
            "entry": entries
        })
    }

    /// Validate FHIR resource (basic validation)
    pub fn validate_observation(&self, resource: &Value) -> bool {
        resource.get("resourceType").and_then(|v| v.as_str()) == Some("Observation")
            && resource.get("status").is_some()
            && resource.get("code").is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn create_test_reading() -> SensorReading {
        SensorReading {
            id: 1,
            device_id: Uuid::new_v4(),
            heart_rate: Some(75),
            spo2: Some(98),
            temperature: Some(36.8),
            reading_timestamp: Utc::now(),
            received_at: Utc::now(),
            quality_score: Some(0.95),
            metadata: json!({}),
        }
    }

    fn create_test_config() -> FhirConfig {
        FhirConfig {
            base_url: "http://localhost:8080/fhir".to_string(),
            organization_id: "org-test-001".to_string(),
        }
    }

    #[test]
    fn test_heart_rate_observation_creation() {
        let service = FhirService::new(create_test_config());
        let reading = create_test_reading();

        let observation = service.create_heart_rate_observation(&reading, None);

        assert_eq!(observation["resourceType"], "Observation");
        assert_eq!(observation["status"], "final");
        assert_eq!(observation["code"]["coding"][0]["code"], "8867-4");
        assert_eq!(observation["valueQuantity"]["value"], 75.0);
    }

    #[test]
    fn test_spo2_observation_creation() {
        let service = FhirService::new(create_test_config());
        let reading = create_test_reading();

        let observation = service.create_spo2_observation(&reading, None);

        assert_eq!(observation["resourceType"], "Observation");
        assert_eq!(observation["code"]["coding"][0]["code"], "2708-6");
        assert_eq!(observation["valueQuantity"]["value"], 98.0);
        assert_eq!(observation["valueQuantity"]["unit"], "percent");
    }

    #[test]
    fn test_temperature_observation_creation() {
        let service = FhirService::new(create_test_config());
        let reading = create_test_reading();

        let observation = service.create_temperature_observation(&reading, None);

        assert_eq!(observation["resourceType"], "Observation");
        assert_eq!(observation["code"]["coding"][0]["code"], "8310-5");
        assert_eq!(observation["valueQuantity"]["value"], 36.8);
    }

    #[test]
    fn test_bundle_creation() {
        let service = FhirService::new(create_test_config());
        let reading = create_test_reading();

        let bundle = service.create_observation_bundle(&reading, Some("Patient/123".to_string()));

        assert_eq!(bundle["resourceType"], "Bundle");
        assert_eq!(bundle["type"], "collection");
        assert_eq!(bundle["entry"].as_array().unwrap().len(), 3); // HR, SpO2, Temp
    }

    #[test]
    fn test_observation_validation() {
        let service = FhirService::new(create_test_config());
        let reading = create_test_reading();

        let observation = service.create_heart_rate_observation(&reading, None);
        assert!(service.validate_observation(&observation));

        let invalid = json!({"resourceType": "Patient"});
        assert!(!service.validate_observation(&invalid));
    }
}
