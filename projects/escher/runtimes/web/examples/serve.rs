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
use axum::http::StatusCode;
use axum::http::Uri;
use axum::response::Html;
use axum::response::Json;
use axum::response::Response;
use axum::response::IntoResponse;
use axum::response::Result as ResultResponse;
use axum::routing::get;
use axum::routing::post;

use serde::Deserialize;

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
            .route("/__escher/create", post(create_scaffold_page))
            .fallback(static_or_resource)
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

/// Serves `.output/pkg/web` first, falling through to `resource_not_found`. Calls `ServeDir`
/// directly rather than using `not_found_service`, which forces every fallback to a 404 status.
/// That breaks excalidraw's `<object>`-based embed rendering for pages that exist and load fine.
async fn static_or_resource(
    State(state): State<ServeExampleState>,
    request: axum::extract::Request,
) -> ResultResponse<Response, ServeExampleError> {
    use tower::ServiceExt;

    let uri = request.uri().clone();

    let static_response = ServeDir::new(PathBuf::from(CARGO_MANIFEST_DIR).join(".output/pkg/web"))
        .append_index_html_on_directories(true)
        .oneshot(request)
        .await
        .expect("ServeDir is infallible")
        .into_response();

    if static_response.status() != StatusCode::NOT_FOUND {
        return Ok(static_response);
    }

    resource_not_found(State(state), uri).await
}

/// Handles any workspace-relative path not found under `.output/pkg/web`. `.html` paths are
/// scaffold pages: served directly if they exist, else an interactive create-prompt. Other
/// missing paths open the excalidraw editor if the file exists, else `404.html`.
#[axum::debug_handler]
pub async fn resource_not_found(
    State(state): State<ServeExampleState>,
    uri: Uri,
) -> ResultResponse<Response, ServeExampleError> {
    let path = &uri.path()[1..];
    let absolute_path = state.workspace_root.join(path);
    let is_html_page = path.ends_with(".html");

    match fs::read_to_string(&absolute_path).await {
        Ok(contents) if is_html_page => {
            if cfg!(all(feature = "dev")) {
                tracing::info!("Serving scaffold page @ {:}", absolute_path.display());
            }
            Ok((StatusCode::OK, Html(contents)).into_response())
        }
        Ok(_contents) => {
            if cfg!(all(feature = "dev")) {
                tracing::info!("Found file @ {:}", absolute_path.display());
            }
            let body = include_str!("../.output/pkg/web/draw.html");
            Ok((StatusCode::OK, Html(body)).into_response())
        }
        Err(_) if is_html_page => {
            if cfg!(all(feature = "dev")) {
                tracing::debug!("Creatable: {:}", absolute_path.display());
            }
            let body = CREATE_PROMPT_TEMPLATE.replace("{{PATH}}", path);
            Ok((StatusCode::NOT_FOUND, Html(body)).into_response())
        }
        Err(_) => {
            if cfg!(all(feature = "dev")) {
                tracing::debug!("Not Found: {:}", absolute_path.display());
            }
            let body = include_str!("../.output/pkg/web/404.html");
            Ok((StatusCode::NOT_FOUND, Html(body)).into_response())
        }
    }
}

#[derive(Deserialize)]
pub struct CreatePageRequest {
    path: String,
    /// A `ScaffoldDescription` JSON value (see `escher_web::description`). Omitted → the built-in
    /// placeholder scaffold.
    #[serde(default)]
    content: Option<serde_json::Value>,
}

/// Writes an SSG-rendered `<escher-scaffold>` page to `path` under the workspace root, creating
/// parent directories as needed. Only `.html` paths without `..` components are accepted. The
/// resolved parent is re-checked against the canonicalized workspace root as a symlink guard.
#[axum::debug_handler]
pub async fn create_scaffold_page(
    State(state): State<ServeExampleState>,
    Json(request): Json<CreatePageRequest>,
) -> ResultResponse<Response, ServeExampleError> {
    let relative_path = request.path.trim_start_matches('/');

    if !relative_path.ends_with(".html") || relative_path.split('/').any(|part| part == "..") {
        return Ok((StatusCode::BAD_REQUEST, "invalid page path").into_response());
    }

    let target = state.workspace_root.join(relative_path);
    let Some(parent) = target.parent() else {
        return Ok((StatusCode::BAD_REQUEST, "invalid page path").into_response());
    };

    fs::create_dir_all(parent).await?;

    let canonical_parent = parent.canonicalize()?;
    if !canonical_parent.starts_with(state.workspace_root.as_path()) {
        return Ok((StatusCode::BAD_REQUEST, "page path escapes workspace").into_response());
    }

    let page_html = render_scaffold_document(request.content.as_ref())
        .map_err(ServeExampleError::Unknown)?;

    fs::write(&target, page_html).await?;

    if cfg!(all(feature = "dev")) {
        tracing::info!("Created scaffold page @ {:}", target.display());
    }

    Ok((StatusCode::OK, Json(CreatePageResponse { created: true })).into_response())
}

/// Renders a complete `.html` page around an `<escher-scaffold>` element. SSG markup gives
/// immediate paint, plus a hydration `<script type="application/json">` payload when `content`
/// is given. `</` is escaped in the embedded JSON to prevent early `<script>` closure.
fn render_scaffold_document(content: Option<&serde_json::Value>) -> Result<String, String> {
    let (fragment, payload_script) = match content {
        Some(value) => {
            let json = serde_json::to_string(value).map_err(|error| error.to_string())?;
            let fragment = escher_web::ssg::render_fragment(&json)?;
            let escaped_json = json.replace("</", "<\\/");
            let script = format!(r#"<script type="application/json">{escaped_json}</script>"#);
            (fragment, script)
        }
        None => (escher_web::ssg::render_default_fragment(), String::new()),
    };

    Ok(format!(
        "<!doctype html>\n\
         <html lang=\"en\">\n\
         <head>\n\
         <meta charset=\"utf-8\" />\n\
         <title>Escher page</title>\n\
         <script type=\"module\" src=\"/scaffold-element.js\"></script>\n\
         </head>\n\
         <body style=\"margin: 0; background: #000;\">\n\
         <escher-scaffold>{fragment}{payload_script}</escher-scaffold>\n\
         </body>\n\
         </html>\n"
    ))
}

#[derive(serde::Serialize)]
struct CreatePageResponse {
    created: bool,
}

const CREATE_PROMPT_TEMPLATE: &str = include_str!("./create_prompt.html");

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
