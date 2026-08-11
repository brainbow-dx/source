// #![allow(unused)]

extern crate alloc;

use std::path::Path;
use std::sync::Arc;
use std::path::PathBuf;
use std::process::ExitCode;

use core::net::SocketAddr;

use derive_more::From;

use eyre::Report;

use uuid::Uuid;

use serde::Serialize;

use clap::Parser;

use tokio::net::TcpListener;
use tokio::fs;

use tower_http::services::ServeDir;

use axum::Router;
use axum::extract::State;
use axum::handler::Handler;
use axum::http::StatusCode;
use axum::http::Uri;
use axum::response::Html;
use axum::response::Json;
use axum::response::Response;
use axum::response::IntoResponse;
use axum::response::Result as ResultResponse;
use axum::routing::get;

#[derive(Default, Debug, Serialize)]
pub struct DevToolsConfig {
    workspace: DevToolsWorkspace,
}

impl DevToolsConfig {
    pub fn new() -> Self {
        DevToolsConfig::default()
    }
    
    pub fn with_workspace(mut self, workspace: DevToolsWorkspace) -> Self {
        self.workspace = workspace;
        self
    }
}

#[derive(Default, Debug, Serialize)]
pub struct DevToolsWorkspace {
    uuid: Uuid,
    root: PathBuf,
}

impl DevToolsWorkspace {
    pub fn new<P: AsRef<Path>>(uuid: Uuid, root: P) -> Self {
        DevToolsWorkspace {
            uuid, // TODO: Namespace??
            root: root.as_ref().to_path_buf(),
        }
    }
}

//---
const CARGO_MANIFEST_DIR: &str = env!("CARGO_MANIFEST_DIR");

//---
#[derive(clap::Parser, Debug)]
#[command(version, about, long_about=None)]
struct Args {
    /// TODO
    #[arg(short, long, default_value = "true")]
    start: bool,

    /// TODO
    #[arg(short, long, default_value = "127.0.0.1:3333")]
    address: String,

    /// TODO
    #[arg(short, long, default_value = CARGO_MANIFEST_DIR)]
    workspace: PathBuf,

    /// TODO
    #[arg(short, long, default_value = "trace")]
    log_level: String,

    /// TODO
    #[arg(long, default_value = "false")]
    console: bool,
}

#[tokio::main]
async fn main() -> Result<ExitCode, Report> {
    let args = Args::parse();

    color_eyre::install()?;
    tracing_subscriber::fmt()
        .with_env_filter(&args.log_level)
        .with_thread_names(false)
        .with_line_number(false)
        .with_target(true)
        .with_file(false)
        .with_ansi(true)
        .without_time()
        .init();

    //--
    if args.start {
        let workspace_root = args.workspace.canonicalize()?.clone();
        let address = args.address.parse::<SocketAddr>()?;

        let state = ServeExampleState {
            workspace_root: Arc::new(workspace_root.clone()),
        };
        
        let wellknown_routes = Router::new()
            .route("/appspecific/com.chrome.devtools.json", get(chrome_devtools_config))
            .with_state(state.clone());

        let app_routes = Router::new()
            .nest("/.well-known", wellknown_routes)
            .fallback_service({
                ServeDir::new(PathBuf::from(CARGO_MANIFEST_DIR).join(".output/pkg/web"))
                    .append_index_html_on_directories(true)
                    .not_found_service(resource_not_found.with_state(state.clone()))
            })
            .with_state(state.clone())
            .into_make_service_with_connect_info::<SocketAddr>();
        
        tracing::debug!("Workspace: {:}", workspace_root.display());
        tracing::debug!("Serving on {:}", address);
        
        let listener = TcpListener::bind(&address).await?;  
        axum::serve(listener, app_routes).await?;
    }
    
    Ok(ExitCode::SUCCESS)
}

#[axum::debug_handler]
async fn chrome_devtools_config(
    State(state): State<ServeExampleState>,
) -> Result<Response, ServeExampleError> {
    let workspace_path = state.workspace_root.as_path();
    let workspace_uuid = Uuid::new_v4();
    
    if cfg!(feature="dev") {
        tracing::debug!("Using Chrome DevTools Workspace#{} @ {}", workspace_uuid, workspace_path.display());
    }
    
    let devtools_config = DevToolsConfig::new()
        .with_workspace(DevToolsWorkspace::new(workspace_uuid, workspace_path));
    
    Ok((StatusCode::OK, Json(devtools_config)).into_response())
}

#[axum::debug_handler]
pub async fn resource_not_found(
    State(state): State<ServeExampleState>,
    uri: Uri,
) -> ResultResponse<Response, ServeExampleError> {
    let absolute_path = state.workspace_root.join(&uri.path()[1..]);
    
    let Ok(_file_contents) = fs::read_to_string(&absolute_path).await else {
        if cfg!(all(feature = "dev")) {
            tracing::debug!("Not Found: {:}", absolute_path.display());
        }
        
        // let body = fs::read(state.workspace_root.join(".output/pkg/web/404.html")).await?;
        let body = include_str!("../.output/pkg/web/404.html");
        return Ok((StatusCode::NOT_FOUND, Html(body)).into_response())
    };
    
    if cfg!(all(feature = "dev")) {
        tracing::info!("Found file @ {:}", absolute_path.display());
    }
    
    // let body = fs::read(state.workspace_root.join(".output/pkg/web/draw.html")).await?;
    let body = include_str!("../.output/pkg/web/draw.html");
    Ok((StatusCode::OK, Html(body)).into_response())
}

#[derive(Clone, Default, Debug)]
pub struct ServeExampleState {
    workspace_root: Arc<PathBuf>,
}

#[derive(oops::Error, From)]
pub enum ServeExampleError {
    #[msg("address error: {0}")]
    AddrParseError(std::net::AddrParseError),
    
    #[msg("uuid error: {0}")]
    UuidError(uuid::Error),
    
    #[msg("eyre error report: {0}")]
    EyreReport(eyre::Report),
    
    #[msg("server error: {0}")]
    ServerError(std::io::Error),
    
    #[msg("unknown error: {0}")]
    Unknown(String),
    
    #[msg("not found: {0}")]
    NotFound(Uri),
}

// TODO: #[derive(atlas::axum::Response)
impl IntoResponse for ServeExampleError {
    fn into_response(self) -> Response {
        // TODO: Get defaults from config?
        let mut status = StatusCode::INTERNAL_SERVER_ERROR;
        let message = String::from("Internal Server Error");
        
        let error_message = match self {
            ServeExampleError::Unknown(message) => message,
            ServeExampleError::AddrParseError(error) => error.to_string(),
            ServeExampleError::UuidError(error) => error.to_string(),
            ServeExampleError::EyreReport(error) => error.to_string(),
            ServeExampleError::ServerError(error) => error.to_string(),
            ServeExampleError::NotFound(uri) => {
                status = StatusCode::NOT_FOUND;
                format!("Resource not found at '{}'.", uri)
            }
        };
        
        if cfg!(all(feature = "dev")) {
            tracing::warn!("serve example error: {}", error_message);
        }
        
        (status, message).into_response()
    }
}
