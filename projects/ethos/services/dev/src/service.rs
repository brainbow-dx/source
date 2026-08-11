use std::convert::Infallible;
use std::future::Future;
use std::pin::Pin;

use axum::body::Body;
use axum::extract::Query;
use axum::http::Request;
use tower::Service;

use axum::http::StatusCode;
use axum::http::request::Parts;
use axum::extract::FromRequestParts;
use axum::response::IntoResponse;
use axum::response::Response;

//---
#[derive(Clone)]
pub struct DevService;

impl DevService {
    pub fn new() -> Self {
        DevService {
            //..
        }
    }
}

impl Service<Request<Body>> for DevService {
    type Response = DevServiceResponse;

    type Error = Infallible;

    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut std::task::Context<'_>) -> std::task::Poll<Result<(), Self::Error>> {
        std::task::Poll::Ready(Ok(()))
    }

    fn call(&mut self, request: Request<Body>) -> Self::Future {
        let response = DevServiceResponse {
            message: format!("TODO"),
        };
        
        Box::pin(async move {
            Ok(response)
        })
    }
}

impl Service<DevServiceRequest> for DevService {
    type Response = DevServiceResponse;

    type Error = DevServiceError;

    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut std::task::Context<'_>) -> std::task::Poll<Result<(), Self::Error>> {
        std::task::Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: DevServiceRequest) -> Self::Future {
        let response = DevServiceResponse {
            message: format!("TODO"),
        };
        
        Box::pin(async move {
            Ok(response)
        })
    }
}

use serde::{Serialize, Deserialize};

#[derive(FromRequestParts)]
pub struct DevServiceRequest {
    #[from_request(via(Query))]
    route: DevServiceRoute,
}

#[derive(Debug, Clone)]
#[derive(Serialize, Deserialize)]
pub enum DevServiceRoute {
    Hello(String),
    Goodbye(String),
}

impl FromRequestParts<DevService> for DevServiceRoute {
    type Rejection = (StatusCode, String);

    async fn from_request_parts(parts: &mut Parts, _state: &DevService) -> Result<Self, Self::Rejection> {
        let header = parts.headers.get("x-service-request")
            .and_then(|v| v.to_str().ok());

        match header {
            Some("hello") => Ok(DevServiceRoute::Hello("Axum User".to_string())),
            Some("goodbye") => Ok(DevServiceRoute::Goodbye("Axum User".to_string())),
            _ => Err((StatusCode::BAD_REQUEST, "Missing or invalid 'x-service-request' header".to_string())),
        }
    }
}

#[derive(Debug)]
pub struct DevServiceResponse {
    pub message: String,
}

impl IntoResponse for DevServiceResponse {
    fn into_response(self) -> Response {
        Response::new(Body::empty())
    }
}

#[derive(oops::Error)]
pub enum DevServiceError {
    #[msg("internal server error: {0}")]
    InternalServerError(String),
}

impl IntoResponse for DevServiceError {
    fn into_response(self) -> Response {
        let error_with_status = match self {
            DevServiceError::InternalServerError(message) => (StatusCode::INTERNAL_SERVER_ERROR, message),
        };

        error_with_status.into_response()
    }
}
