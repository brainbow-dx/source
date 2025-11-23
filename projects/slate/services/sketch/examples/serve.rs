#![allow(unused)]

use std::path::PathBuf;
use std::process::ExitCode;

use core::net::SocketAddr;
use std::sync::Arc;

use eyre::Result;

use clap::Parser;

use tokio::fs;
use tokio::net::TcpListener;

use tower_http::services::ServeDir;

use axum::Router;
use axum::body::Body;
use axum::extract::ConnectInfo;
use axum::extract::Extension;
use axum::extract::Path;
use axum::extract::Request;
use axum::extract::State;
use axum::http::StatusCode;
use axum::http::Uri;
use axum::middleware::Next;
use axum::middleware::from_fn;
use axum::middleware::from_fn_with_state;
use axum::response::Html;
use axum::response::IntoResponse;
use axum::response::Response;
use axum::routing::get;

const CARGO_MANIFEST_DIR: &str = env!("CARGO_MANIFEST_DIR");

const DEFAULT_SKETCH_ADDR: &str = "127.0.0.1:3000";

//---
#[derive(clap::Parser, Debug)]
#[command(version, about, long_about=None)]
struct Args {
    /// TODO
    #[arg(short, long, default_value = "true")]
    start: bool,

    /// TODO
    #[arg(short, long, default_value=DEFAULT_SKETCH_ADDR)]
    address: String,

    /// TODO
    #[arg(short, long, default_value = "trace")]
    log_level: String,

    /// TODO
    #[arg(long, default_value = "false")]
    console: bool,
}

#[tokio::main]
async fn main() -> Result<ExitCode> {
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
    let example = ServeExample::new(CARGO_MANIFEST_DIR);

    if args.start {
        example.start(&args.address).await?;
    }

    Ok(ExitCode::SUCCESS)
}

#[derive(Default, Debug)]
pub struct ServeExample {
    root: PathBuf,
}

#[derive(Default, Debug)]
pub struct ServeExampleState {
    workdir: PathBuf,
}

impl ServeExample {
    pub fn new<P: Into<PathBuf>>(root: P) -> Self {
        ServeExample {
            root: root.into(),
        }
    }
}

impl ServeExample {
    pub async fn start<A: AsRef<str>>(&self, address: A) -> Result<()> {
        let scheme = "http"; // TODO: Get this from state/config.
        let address = address.as_ref().parse::<SocketAddr>()?;
        let listener = TcpListener::bind(&address).await?;

        let state = Arc::new(ServeExampleState {
            workdir: self.root.clone(),
        });

        let public_assets = ServeDir::new(self.root.join(".output/pkg/web/public"))
            .append_index_html_on_directories(true)
            .not_found_service(get(load_workspace_resource));

        let router = Router::new()
            .route("/", get(index))
            // .route("/{*resource_path}", get(load_workspace_resource))
            .fallback_service(public_assets)
            .with_state(state);

        let app = router.into_make_service_with_connect_info::<SocketAddr>();

        tracing::debug!("Listening on {:}://{:}", scheme, address);
        tracing::info!("Serving at {:}://{:}", scheme, "localhost:3000");

        axum::serve(listener, app).await?;

        Ok(())
    }
}

async fn index(ConnectInfo(address): ConnectInfo<SocketAddr>) -> impl IntoResponse {
    tracing::info!("Got request from address: {}", address);

    let index_page_content = include_str!("../.output/pkg/web/public/default.html");
    Html(index_page_content.to_owned())
}

async fn load_workspace_resource(
    // State(state): State<Arc<ServeExampleState>>,
    // Path(resource_path): Path<String>,
    // request: Request<Body>,
    uri: Uri,
) -> impl IntoResponse {
    let draw_page_content = String::from(include_str!("../.output/pkg/web/public/draw.html"));

    // TODO: Get the workdir from ServeExampleState ..
    let workdir = PathBuf::from(CARGO_MANIFEST_DIR);

    if let Some(resource_path) = uri.path().strip_prefix('/') {
        match fs::metadata(workdir.join(resource_path)).await {
            Ok(resource) =>
                if resource.is_file() {
                    return match fs::read(workdir.join(resource_path)).await {
                        Ok(bytes) => {
                            // TODO: let content = Body::from(bytes);
                            (StatusCode::OK, Html(draw_page_content))
                        }
                        Err(error) => (StatusCode::INTERNAL_SERVER_ERROR, Html(draw_page_content)),
                    };
                },
            Err(error) => {
                tracing::error!("Failed to get metadata for file '{:}': {:}", resource_path, error);
            }
        }
    }

    (StatusCode::OK, Html(draw_page_content))
}

async fn workspace_resource_not_found(ConnectInfo(address): ConnectInfo<SocketAddr>, uri: Uri) -> impl IntoResponse {
    let missing_page_content = include_str!("../.output/pkg/web/public/draw.html");

    if cfg!(all(feature = "dev", feature = "verbose")) {
        tracing::trace!("Sometime before the Make America Great Depression ..");
        tracing::debug!("Connection '{:}' requested resource:\n{:#?}", address, uri);
        tracing::debug!("Not Found: {:?}", uri);
    }

    (StatusCode::OK, Html(missing_page_content.to_owned()))
}
