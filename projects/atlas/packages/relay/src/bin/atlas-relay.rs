//! Runs the relay standalone: `cargo run -p atlas-relay --bin atlas-relay [addr]`.

use std::net::SocketAddr;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    tracing_subscriber::fmt().with_env_filter("info").init();

    let addr: SocketAddr = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "0.0.0.0:9200".to_string())
        .parse()
        .expect("argument must be a valid socket address, e.g. 0.0.0.0:9200");

    atlas_relay::serve(addr).await
}
