use std::path::Path;
use std::process::ExitCode;

use eyre::Result;

use notify::{Error, EventKind};
use notify::{RecursiveMode, Watcher};

use tokio::process::Command;
use tokio::net::TcpListener;
use tokio::sync::broadcast;
use tokio::sync::mpsc;

use tower_http::services::ServeDir;

use axum::Router;
use axum::Extension;
use axum::response::IntoResponse;
use axum::extract::WebSocketUpgrade;
use axum::extract::ws::WebSocket;
use axum::extract::ws::Message;
use axum_extra::TypedHeader;
use axum_extra::headers::UserAgent;

#[tokio::main]
async fn main() -> Result<ExitCode> {
    tracing_subscriber::fmt::init();
    
    // let app_dir = std::fs::canonicalize(web_test::APP_DIR)?;
    let public_dir = std::fs::canonicalize("public")?;
    
    let (broadcast_tx, _) = broadcast::channel::<String>(16);
    
    let watch_thread = {
        let public_dir = public_dir.clone();
        let broadcast_tx = broadcast_tx.clone();
        tokio::spawn(async move {
            watch_files(&public_dir, broadcast_tx).await
        })
    };
    
    let mut api_service_thread = {
        Command::new("cargo")
            .args(["run", "-p", "web-test-api"])
            .spawn()?
    };
    
    let public_dir = {
        axum::routing::get_service({
            ServeDir::new(public_dir)
                .append_index_html_on_directories(true)
                // .not_found_service(service_fn(|req| async move {
                //     (
                //         StatusCode::NOT_FOUND,
                //         format!("Something went wrong: {error}"),
                //     )
                // }))
        })
    };
    
    let app = Router::new()
        .route("/__/ws", axum::routing::get(ws_handler))
        .fallback_service(public_dir)
        .layer(Extension(broadcast_tx));
    
    let host = "localhost";
    let port = 3000;
    tracing::info!("Server running at http://{}:{}", host, port);
    
    let listener = TcpListener::bind((host, port)).await?;
    
    if let Err(error) = axum::serve(listener, app).await {
        tracing::error!("dev service failed: {:}", error);
    };
    
    if let Some(_pid) = api_service_thread.id() {
        tracing::debug!("Shutting down child process id {:}", _pid);
        api_service_thread.kill().await?;
    }
    
    if !watch_thread.is_finished() {
        watch_thread.abort();
    }
    
    Ok(ExitCode::SUCCESS)
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    _user_agent: Option<TypedHeader<UserAgent>>,
    // ConnectInfo(addr): ConnectInfo<std::net::SocketAddr>,
    Extension(broadcast_tx): Extension<broadcast::Sender<String>>,
) -> impl IntoResponse {
    tracing::debug!("User connected.");
    
    #[cfg(any(feature="dev"))]
    if let Some(TypedHeader(user_agent)) = _user_agent {
        tracing::debug!("User-Agent: {:}", user_agent);
    }
    
    // tracing::debug!("`{user_agent:?}` at {addr:?} connected.");
    // finalize the upgrade process by returning upgrade callback.
    // we can customize the callback by sending additional info such as address.
    ws.on_upgrade(move |socket| handle_socket(socket, broadcast_tx.subscribe()))
}

async fn handle_socket(mut socket: WebSocket, mut fs_watcher: broadcast::Receiver<String>) {
    #[cfg(any(feature="dev"))]
    println!("WebSocket connection established!");
    
    let _session_report = loop {
        match fs_watcher.recv().await {
            Ok(message) => {
                if let Err(error) = socket.send(Message::Text(message.into())).await {
                    eprintln!("Failed to send update to user: {:?}", error);
                    break "Failed on ws send.";
                }
            }
            Err(error) => {
                eprintln!("Failed to get fs update: {}", error);
                break "Failed on fs recv.";
            }
        }
    };
    
    #[cfg(any(feature="dev"))]
    println!("WebSocket connection closed; Report: {:}", _session_report);
}

// Watch files in the `static` directory for changes
async fn watch_files(watch_dir: &Path, broadcast_tx: broadcast::Sender<String>) -> Result<(), Error> {
    let (notify_tx, mut notify_rx) = mpsc::unbounded_channel();
    let mut watcher = notify::recommended_watcher(move |res| {
        if let Ok(event) = res {
            notify_tx.send(event).expect("send fs notification");
        }
    })?;

    watcher.watch(watch_dir, RecursiveMode::Recursive)?;

    while let Some(event) = notify_rx.recv().await {
        if let Some(path) = event.paths.first() {
            if let EventKind::Modify(_) = event.kind {
                #[cfg(all(feature="dev"))]
                println!("Changed {:?}", path.to_string_lossy().to_string());
                
                let relative_path = path.strip_prefix(watch_dir).unwrap_or(path);
                let asset_path = relative_path.to_string_lossy().to_string();
                broadcast_tx.send(asset_path).unwrap_or(0);
            }
        }
    }
    
    Ok(())
}