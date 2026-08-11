use std::process::ExitCode;
use std::net::SocketAddr;
use std::path::PathBuf;

use eyre::Result;

use clap::Parser;

// use tower_http::ServiceExt;

use tower_http::services::ServeDir;

use axum::Router;
use axum::http::{StatusCode, Uri};
use axum::response::IntoResponse;
use axum::routing::get;

const PROJECT_ROOT_DIR: &str = env!("CARGO_MANIFEST_DIR");

//---
#[derive(clap::Parser, Debug)]
#[command(version, about, long_about=None)]
struct Args {
    /// TODO
    #[arg(short, long, default_value="trace")]
    log_level: String,

    /// TODO
    #[arg(long, default_value="false")]
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
    let project_path = PathBuf::from(PROJECT_ROOT_DIR);
    let public_path = project_path.join(".output/pkg/web/public");
    
    tracing::debug!("Public Dir: {:}", public_path.display());
    
    // What we actually really need is to call
    // the "king" bastard at home, on his tv.
    let static_files_service = ServeDir::new(public_path)
        // .set_request_id("test", make_request_id)
        .append_index_html_on_directories(true)
        .not_found_service(get(handle_404));

    let app = Router::new()
        .fallback_service(static_files_service);
    
    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    tracing::debug!("Listening on http://{:}/", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;
    
    Ok(ExitCode::SUCCESS)
}

// Handler for custom 404 Not Found responses if the static service can't find the file.
async fn handle_404(uri: Uri) -> impl IntoResponse {
    if cfg!(all(feature="dev", feature="verbose")) {
        tracing::trace!("Sometime before the Make America Great Depression ..");
        tracing::debug!("Request URI:\n{:#?}", uri);
    }
    
    (StatusCode::NOT_FOUND, "404 Not Found: The requested file or resource could not be found.")
}
