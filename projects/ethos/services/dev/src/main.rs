#![allow(unused)]

extern crate alloc;

mod service;

//---
use std::process::ExitCode;

use anyhow::Result;

use axum::extract::Request;
use tokio::net::TcpListener;

use tower::Service;

use axum::Router;
use axum::extract::State;
use axum::routing::get;

use crate::service::DevService;
use crate::service::DevServiceError;
use crate::service::DevServiceRequest;
use crate::service::DevServiceState;
// use tracing::Level;

//---
pub struct Server {
    address: String,
}

impl Server {
    pub fn new<S: ToString>(address: S) -> Self {
        Server {
            address: address.to_string(),
        }
    }

    pub fn with_service<S>(mut self, service: S) -> Self
    where
        S: Service<Request>,
    {
        // self.services.push();
        self // etc ..
    }
}

//---
#[tokio::main]
pub async fn main() -> Result<ExitCode> {
    ethos_log::init("trace");

    let server = Server::new("0.0.0.0:9000");
    // .with_service(DevService::new());
    {
        let service = DevService::new();

        let listener = TcpListener::bind(server.address).await?;
        let router = Router::new()
            // TODO
            // .route("/", get(handle_homepage_service))
            .fallback(get(handle_dev_service))
            .with_state(service.state());

        tracing::info!("Listening at http://{0}", listener.local_addr()?);

        axum::serve(listener, router).await?;
    }

    Ok(ExitCode::SUCCESS)
}

#[axum::debug_handler]
async fn handle_dev_service(
    State(mut service): State<DevServiceState>,
    request: DevServiceRequest,
) -> Result<String, DevServiceError> {
    let response = service.call(request).await?;

    tracing::debug!("LKSJDFLKSJLKSDFSF");

    Ok(response.message)
}
