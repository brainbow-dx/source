// #![allow(unused)]

use std::path::PathBuf;
use std::process::ExitCode;

use core::net::SocketAddr;
use std::sync::Arc;

use axum::extract::State;
use axum::handler::Handler;
use eyre::Result;

use clap::Parser;

use tokio::fs;
use tokio::net::TcpListener;

use tower_http::services::ServeDir;

use axum::Router;
use axum::body::Body;
use axum::extract::ConnectInfo;
use axum::extract::Request;
use axum::http::StatusCode;
use axum::http::Uri;
use axum::middleware::Next;
use axum::middleware::from_fn_with_state;
use axum::response::Html;
use axum::response::IntoResponse;
use axum::response::Response;

const CARGO_MANIFEST_DIR: &str = env!("CARGO_MANIFEST_DIR");

const DEFAULT_SKETCH_ADDR: &str = "127.0.0.1:3000";

//---
#[derive(clap::Parser, Debug)]
#[command(version, about, long_about=None)]
struct Args {
    /// TODO
    #[arg(long, default_value = CARGO_MANIFEST_DIR)]
    cwd: PathBuf,
    
    /// TODO
    #[arg(long, default_value = "true")]
    start: bool,

    /// TODO
    #[arg(long, default_value = DEFAULT_SKETCH_ADDR)]
    address: String,
    
    /// TODO
    #[arg(long, default_value = "trace")]
    log_filter: String,

    /// TODO
    #[arg(long, default_value = "false")]
    terminal: bool,
}

#[tokio::main]
async fn main() -> Result<ExitCode> {
    let args = Args::parse();

    color_eyre::install()?;

    tracing_subscriber::fmt()
        .with_env_filter(&args.log_filter)
        .with_thread_names(false)
        .with_line_number(false)
        .with_target(true)
        .with_file(false)
        .with_ansi(true)
        .without_time()
        .init();

    //--
    if args.start {
        let workspace_root = args.cwd.clone().canonicalize()?;
        let address = args.address.parse::<SocketAddr>()?;

        #[cfg(feature="dev")]
        tracing::debug!("Workspace Root: {:}", workspace_root.display());
        
        let state = ServeExampleState {
            workdir: Arc::new(workspace_root),
        };

        let router = Router::new()
            // .route("/{*resource_path}", get(load_workspace_resource))
            .fallback_service({
                ServeDir::new(state.workdir.join(".output/pkg/web"))
                    .append_index_html_on_directories(true)
                    .not_found_service(resource_not_found.with_state(state.clone()))
            })
            .with_state(state.clone())
            .layer(from_fn_with_state(state.clone(), load_workspace_resource))
            .into_make_service_with_connect_info::<SocketAddr>();
        
        tracing::info!("Serving on {:}", address);
        
        axum::serve(TcpListener::bind(&address).await?, router).await?;
    }

    Ok(ExitCode::SUCCESS)
}

#[derive(Clone, Default, Debug)]
pub struct ServeExampleState {
    workdir: Arc<PathBuf>,
}

pub async fn load_workspace_resource(
    request: Request<Body>,
    next: Next,
    // State(_state): State<ServeExampleState>,
) -> Response {
    // TODO: Get cwd from extension(s) or state.
    let cwd = PathBuf::from(CARGO_MANIFEST_DIR);
    
    let Some(request_path) = request.uri().path().strip_prefix("/") else {
        return StatusCode::BAD_REQUEST.into_response()
    };
    
    match fs::try_exists(cwd.join(request_path)).await {
        Ok(true) => {
            match fs::read(cwd.join(".output/pkg/web/draw.html")).await {
                Ok(page_content) => {
                    // TODO: Unpack it ..
                    Html::from(page_content).into_response()
                }
                Err(error) => {
                    tracing::error!("Failed to load page content: {}", error);
                    StatusCode::INTERNAL_SERVER_ERROR.into_response()
                }
            }
        }
        Err(error) => {
            tracing::error!("Failed to get metadata for resource '{}': {}", cwd.join(request_path).display(), error);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
        _ => {
            next.run(request).await
        }
    }
}

pub async fn resource_not_found(
    state: State<ServeExampleState>,
    address: ConnectInfo<SocketAddr>,
    uri: Uri,
) -> Response {
    if cfg!(all(feature = "dev")) {
        let ConnectInfo(address) = address;
        tracing::debug!("Connection '{:}' requested resource:\n{:#?}", address, uri);
        tracing::debug!("Not Found: {:?}", uri);
    }
    
    match fs::read(state.workdir.join(".output/pkg/web/404.html")).await {
        Ok(page_content) => {
            (StatusCode::NOT_FOUND, Html(page_content)).into_response()
        }
        Err(error) => {
            tracing::error!("Failed to load 404 page content: {}", error);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}
