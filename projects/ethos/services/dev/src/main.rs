#![allow(unused)]

extern crate alloc;

mod service;

use std::any::Any;
use std::any::TypeId;
//---
use std::process::ExitCode;
use std::sync::Arc;

use anyhow::Result;

use axum::extract::Request;
use dashmap::DashMap;
use serde::Deserialize;
use serde::Serialize;
use tokio::net::TcpListener;

use tower::Service;

use axum::Router;
use axum::extract::State;
use axum::routing::get;

use crate::service::DevService;
use crate::service::DevServiceRequest;
use crate::service::DevServiceError;

//---
#[derive(Clone)]
pub struct DevServer {
    address: String,
    state: DevServerState,
}

#[derive(Clone, Default)]
// #[derive(Serialize, Deserialize)]
pub struct DevServerState {
    //..
}

impl DevServer {
    pub fn new<A: ToString>(address: A) -> Self {
        DevServer {
            address: address.to_string(),
            state: DevServerState::default(),
        }
    }
    
    pub fn with_state(mut self, state: DevServerState) -> Self {
        self.state = state;
        self // ..
    }

    pub fn state(&self) -> DevServerState {
        self.state.clone()
    }
    
    pub async fn start(&self) -> Result<()> {
        let listener = TcpListener::bind(&self.address).await?;
        let router = Router::new()
            // TODO
            .route_service("/", DevService::new())
            .fallback(get(handle_not_found))
            .with_state(self.state());
        
        tracing::info!("Listening (Dev) at http://{0}", listener.local_addr()?);

        axum::serve(listener, router).await?;
        
        Ok(())
    }
}

//---
#[tokio::main]
pub async fn main() -> Result<ExitCode> {
    ethos_log::init("trace");

    let server = DevServer::new("0.0.0.0:9000");
    
    server.start().await?;

    Ok(ExitCode::SUCCESS)
}

//--
#[derive(Clone, Default)]
#[derive(Serialize, Deserialize)]
pub struct ServeExample {
    //..
}

#[axum::debug_handler]
async fn handle_not_found(
    State(mut state): State<DevServerState>,
    request: DevServiceRequest,
) -> Result<String, DevServiceError> {
    // let response = state.call(request).await?;

    tracing::debug!("LKSJDFLKSJLKSDFSF");

    // Ok(response.message)
    Ok(format!("TODO"))
}
