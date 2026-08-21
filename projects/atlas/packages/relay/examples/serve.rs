//! Runs the relay standalone, for local dev/testing only (`compose.yaml`'s `relay` service runs
//! this): `cargo run --example serve -p atlas-relay [addr]`. `atlas-relay` itself ships no bin
//! target — real consumers embed `atlas_relay::serve` directly on their own runtime (see
//! `apps/anvil/src/main.rs`), the same way this example does.

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
