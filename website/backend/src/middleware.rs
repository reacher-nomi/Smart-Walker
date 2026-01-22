use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error, HttpMessage,
};
use futures::future::LocalBoxFuture;
use std::future::{ready, Ready};
use std::rc::Rc;
use tracing::{info, warn};

/// Audit logging middleware for HIPAA compliance
pub struct AuditLogger;

impl<S, B> Transform<S, ServiceRequest> for AuditLogger
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = AuditLoggerMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(AuditLoggerMiddleware {
            service: Rc::new(service),
        }))
    }
}

pub struct AuditLoggerMiddleware<S> {
    service: Rc<S>,
}

impl<S, B> Service<ServiceRequest> for AuditLoggerMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let svc = self.service.clone();
        
        Box::pin(async move {
            let method = req.method().to_string();
            let path = req.path().to_string();
            let ip = req.peer_addr().map(|addr| addr.to_string());
            let user_agent = req.headers()
                .get("user-agent")
                .and_then(|h| h.to_str().ok())
                .map(|s| s.to_string());

            // Extract user info if available (from JWT middleware)
            let user_email = req.extensions().get::<String>().cloned();

            let start_time = std::time::Instant::now();
            let res = svc.call(req).await;
            let elapsed = start_time.elapsed();

            match &res {
                Ok(response) => {
                    let status = response.status().as_u16();
                    
                    // Log all API requests
                    if path.starts_with("/api/") || path.starts_with("/auth/") {
                        info!(
                            method = %method,
                            path = %path,
                            status = status,
                            duration_ms = elapsed.as_millis(),
                            ip = ?ip,
                            user = ?user_email,
                            user_agent = ?user_agent,
                            "API_REQUEST"
                        );
                    }

                    // Log authentication events
                    if path.starts_with("/auth/") {
                        info!(
                            event_type = "authentication",
                            action = path.trim_start_matches("/auth/"),
                            status = status,
                            user = ?user_email,
                            ip = ?ip,
                            "AUTH_EVENT"
                        );
                    }

                    // Log data access (HIPAA requirement)
                    if path.contains("/vitals") || path.contains("/fhir") {
                        info!(
                            event_type = "data_access",
                            resource = path,
                            user = ?user_email,
                            status = status,
                            "DATA_ACCESS"
                        );
                    }
                }
                Err(err) => {
                    warn!(
                        method = %method,
                        path = %path,
                        error = %err,
                        ip = ?ip,
                        "REQUEST_ERROR"
                    );
                }
            }

            res
        })
    }
}

/// Request ID middleware for tracing
pub struct RequestId;

impl<S, B> Transform<S, ServiceRequest> for RequestId
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = RequestIdMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(RequestIdMiddleware {
            service: Rc::new(service),
        }))
    }
}

pub struct RequestIdMiddleware<S> {
    service: Rc<S>,
}

impl<S, B> Service<ServiceRequest> for RequestIdMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let request_id = uuid::Uuid::new_v4().to_string();
        req.extensions_mut().insert(request_id.clone());

        let svc = self.service.clone();
        
        Box::pin(async move {
            let mut res = svc.call(req).await?;
            res.headers_mut().insert(
                actix_web::http::header::HeaderName::from_static("x-request-id"),
                actix_web::http::header::HeaderValue::from_str(&request_id).unwrap(),
            );
            Ok(res)
        })
    }
}
