use std::future::Future;
use std::pin::Pin;

use tower::Service;

use axum::extract::FromRequestParts;
use axum::http::StatusCode;
use axum::http::request::Parts;
use axum::response::IntoResponse;
use axum::response::Response;

//---
pub struct DevService {
    state: DevServiceState,
}

impl DevService {
    pub fn new() -> Self {
        let state = DevServiceState::new();

        DevService {
            state,
        }
    }

    pub fn state(&self) -> DevServiceState {
        self.state.clone()
    }
}

#[derive(Clone)]
pub struct DevServiceState;

impl DevServiceState {
    pub fn new() -> Self {
        DevServiceState {
            //..
        }
    }
}

impl Service<DevServiceRequest> for DevServiceState {
    type Response = DevServiceResponse;

    type Error = DevServiceError;

    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut std::task::Context<'_>) -> std::task::Poll<Result<(), Self::Error>> {
        std::task::Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: DevServiceRequest) -> Self::Future {
        let response = match req {
            DevServiceRequest::Hello(name) => DevServiceResponse {
                message: format!("Hello, {}!", name),
            },
            DevServiceRequest::Goodbye(name) => DevServiceResponse {
                message: format!("Goodbye, {}!", name),
            },
        };

        Box::pin(async move { Ok(response) })
    }
}

#[derive(Debug, Clone)]
pub enum DevServiceRequest {
    Hello(String),
    Goodbye(String),
}

impl FromRequestParts<DevServiceState> for DevServiceRequest {
    type Rejection = (StatusCode, String);

    async fn from_request_parts(parts: &mut Parts, _state: &DevServiceState) -> Result<Self, Self::Rejection> {
        let header = parts.headers.get("x-service-request").and_then(|v| v.to_str().ok());

        match header {
            Some("hello") => Ok(DevServiceRequest::Hello("Axum User".to_string())),
            Some("goodbye") => Ok(DevServiceRequest::Goodbye("Axum User".to_string())),
            _ => Err((StatusCode::BAD_REQUEST, "Missing or invalid 'x-service-request' header".to_string())),
        }
    }
}

#[derive(Debug)]
pub struct DevServiceResponse {
    pub message: String,
}

impl FromRequestParts<()> for DevServiceRequest {
    type Rejection = (StatusCode, String);

    async fn from_request_parts(parts: &mut Parts, _state: &()) -> Result<Self, Self::Rejection> {
        let header = parts.headers.get("x-service-request").and_then(|v| v.to_str().ok());

        match header {
            Some("hello") => Ok(DevServiceRequest::Hello("Axum User".to_string())),
            Some("goodbye") => Ok(DevServiceRequest::Goodbye("Axum User".to_string())),
            _ => Err((StatusCode::BAD_REQUEST, "Missing or invalid 'x-service-request' header".to_string())),
        }
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
