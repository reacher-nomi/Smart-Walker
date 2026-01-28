use crate::models::{LatestVitals, MlAlert, SseEvent};
use actix_web::{web, HttpResponse, Responder};
use async_stream::stream;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;
use tokio::time::interval;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;

/// Broadcast channel for SSE events
pub type SseBroadcaster = Arc<broadcast::Sender<SseEvent>>;

/// Create a new SSE broadcaster
pub fn create_broadcaster() -> SseBroadcaster {
    let (tx, _rx) = broadcast::channel::<SseEvent>(100);
    Arc::new(tx)
}

/// SSE event handler - streams vitals to frontend
pub async fn stream_vitals(
    broadcaster: web::Data<SseBroadcaster>,
) -> impl Responder {
    let rx = broadcaster.subscribe();
    let stream = BroadcastStream::new(rx);

    let event_stream = stream! {
        // Send initial heartbeat
        yield Ok::<_, actix_web::Error>(
            web::Bytes::from(format!("event: heartbeat\ndata: {}\n\n", 
                serde_json::to_string(&serde_json::json!({"timestamp": chrono::Utc::now().timestamp()})).unwrap()
            ))
        );

        tokio::pin!(stream);

        // Send heartbeat every 30 seconds + forward all events
        let mut heartbeat_interval = interval(Duration::from_secs(30));
        
        loop {
            tokio::select! {
                _ = heartbeat_interval.tick() => {
                    let heartbeat = SseEvent::Heartbeat { 
                        timestamp: chrono::Utc::now().timestamp() 
                    };
                    if let Ok(json) = serde_json::to_string(&heartbeat) {
                        yield Ok::<_, actix_web::Error>(
                            web::Bytes::from(format!("event: heartbeat\ndata: {}\n\n", json))
                        );
                    }
                }
                Some(msg) = stream.next() => {
                    match msg {
                        Ok(event) => {
                            let (event_type, data) = match &event {
                                SseEvent::Vitals { data } => ("vitals", serde_json::to_string(data)),
                                SseEvent::Alert { data } => ("alert", serde_json::to_string(data)),
                                SseEvent::Heartbeat { timestamp } => ("heartbeat", serde_json::to_string(&serde_json::json!({"timestamp": timestamp}))),
                            };

                            if let Ok(json) = data {
                                yield Ok::<_, actix_web::Error>(
                                    web::Bytes::from(format!("event: {}\ndata: {}\n\n", event_type, json))
                                );
                            }
                        }
                        Err(_) => {
                            // Channel closed or lagged, break the loop
                            break;
                        }
                    }
                }
            }
        }
    };

    HttpResponse::Ok()
        .content_type("text/event-stream")
        .insert_header(("Cache-Control", "no-cache"))
        .insert_header(("X-Accel-Buffering", "no"))
        .streaming(event_stream)
}

/// Broadcast a vitals update to all SSE clients
pub fn broadcast_vitals(broadcaster: &SseBroadcaster, vitals: LatestVitals) {
    let event = SseEvent::Vitals { data: vitals };
    let _ = broadcaster.send(event);
}

/// Broadcast an ML alert to all SSE clients
pub fn broadcast_alert(broadcaster: &SseBroadcaster, alert: MlAlert) {
    let event = SseEvent::Alert { data: alert };
    let _ = broadcaster.send(event);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_broadcaster_creation() {
        let broadcaster = create_broadcaster();
        assert_eq!(broadcaster.receiver_count(), 0);
    }

    #[test]
    fn test_broadcast_vitals() {
        let broadcaster = create_broadcaster();
        let mut rx = broadcaster.subscribe();

        let vitals = LatestVitals {
            heartRate: 75,
            spo2: 98,
            temperature: 36.8,
            timestamp: 1234567890,
            quality_score: Some(0.95),
            ml_alert: None,
        };

        broadcast_vitals(&broadcaster, vitals.clone());

        // Try to receive the event
        let result = rx.try_recv();
        assert!(result.is_ok());

        if let Ok(SseEvent::Vitals { data }) = result {
            assert_eq!(data.heartRate, 75);
        }
    }

    #[test]
    fn test_broadcast_alert() {
        let broadcaster = create_broadcaster();
        let mut rx = broadcaster.subscribe();

        let alert = MlAlert {
            level: "critical".to_string(),
            message: "Test alert".to_string(),
            details: serde_json::json!({}),
        };

        broadcast_alert(&broadcaster, alert.clone());

        let result = rx.try_recv();
        assert!(result.is_ok());

        if let Ok(SseEvent::Alert { data }) = result {
            assert_eq!(data.level, "critical");
        }
    }
}
