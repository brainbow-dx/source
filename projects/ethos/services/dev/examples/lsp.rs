use std::convert::Infallible;
use std::future::Future;
use std::pin::Pin;

use tokio::net::TcpListener;

use tower::Service;

use axum::Router;
use axum::extract::FromRequestParts;
use axum::extract::State;
use axum::http::StatusCode;
use axum::http::request::Parts;
use axum::routing::get;

// ---
// Dependencies for Cargo.toml:
//
// [dependencies]
// axum = "0.7"
// tower = "0.4"
// tokio = { version = "1.0", features = ["full"] }
// tracing = "0.1"
// tracing-subscriber = "0.3"
// ---

/// A simple enum representing different service requests.
#[derive(Debug, Clone)]
enum MyServiceRequest {
    Hello(String),
    Goodbye(String),
}

/// The response type for our service.
#[derive(Debug)]
struct MyServiceResponse {
    message: String,
}

/// A simple struct that implements the Tower Service trait for our enum.
/// This struct holds no state, but it could if needed.
#[derive(Clone)]
struct MyTowerService;

impl Service<MyServiceRequest> for MyTowerService {
    // The type of the request this service accepts.
    type Response = MyServiceResponse;
    // The type of errors that can be returned.
    type Error = Infallible;
    // The future returned by `call`.
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut std::task::Context<'_>) -> std::task::Poll<Result<(), Self::Error>> {
        // This service is always ready to accept requests.
        std::task::Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: MyServiceRequest) -> Self::Future {
        // The service logic: we match on the enum to determine the response.
        let response = match req {
            MyServiceRequest::Hello(name) => MyServiceResponse {
                message: format!("Hello, {}!", name),
            },
            MyServiceRequest::Goodbye(name) => MyServiceResponse {
                message: format!("Goodbye, {}!", name),
            },
        };

        // Return the response wrapped in a future.
        Box::pin(async move { Ok(response) })
    }
}

/// An Axum extractor to turn a request into our enum type.
/// In a real-world scenario, you would probably parse a header or a query
/// string to determine which enum variant to use.
// #[async_trait::async_trait]
impl FromRequestParts<MyTowerService> for MyServiceRequest {
    type Rejection = (StatusCode, String);

    async fn from_request_parts(parts: &mut Parts, _state: &MyTowerService) -> Result<Self, Self::Rejection> {
        // This is a simplified example. We'll check for a custom header
        // to decide which enum variant to create.
        let header = parts.headers.get("x-service-request").and_then(|v| v.to_str().ok());

        match header {
            Some("hello") => Ok(MyServiceRequest::Hello("Axum User".to_string())),
            Some("goodbye") => Ok(MyServiceRequest::Goodbye("Axum User".to_string())),
            _ => Err((StatusCode::BAD_REQUEST, "Missing or invalid 'x-service-request' header".to_string())),
        }
    }
}

/// An Axum extractor to turn a request into our enum type.
/// In a real-world scenario, you would probably parse a header or a query
/// string to determine which enum variant to use.
// #[async_trait::async_trait]
impl FromRequestParts<()> for MyServiceRequest {
    type Rejection = (StatusCode, String);

    async fn from_request_parts(parts: &mut Parts, _state: &()) -> Result<Self, Self::Rejection> {
        // This is a simplified example. We'll check for a custom header
        // to decide which enum variant to create.
        let header = parts.headers.get("x-service-request").and_then(|v| v.to_str().ok());

        match header {
            Some("hello") => Ok(MyServiceRequest::Hello("Axum User".to_string())),
            Some("goodbye") => Ok(MyServiceRequest::Goodbye("Axum User".to_string())),
            _ => Err((StatusCode::BAD_REQUEST, "Missing or invalid 'x-service-request' header".to_string())),
        }
    }
}

// Our Axum handler function that uses the custom enum extractor.
async fn handler(
    State(mut service): State<MyTowerService>,
    request: MyServiceRequest,
) -> Result<String, (StatusCode, String)> {
    // Call our Tower service with the extracted request.
    let response = service
        .call(request)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("{e:?}")))?;

    // Return the response message as the HTTP body.
    Ok(response.message)
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    // Create a new instance of our custom Tower service.
    let my_service = MyTowerService;

    // Use `ServiceBuilder` to wrap our service.
    let app = Router::new().route("/", get(handler)).with_state(my_service);

    let listener = TcpListener::bind("127.0.0.1:3000").await.unwrap();
    println!("Listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}
