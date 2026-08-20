//! A real (not stubbed) `libsql` remote-replica sync example: opens a local embedded replica,
//! syncs it against a running `sqld` (see `tools/data/compose.yaml` in the monorepo root —
//! `docker compose -f tools/data/compose.yaml up -d sqld`), appends a guestbook entry from
//! stdin, syncs again, and prints every entry synced from the remote so far.

use anyhow::Result;

use libsql::Builder;
use libsql::params;

const DEFAULT_SYNC_URL: &str = "http://localhost:8081";

#[tokio::main]
async fn main() -> Result<()> {
    atlas::log::init("INFO,atlas_examples_sync=TRACE,atlas=TRACE");

    let data_dir = atlas::env::get_data_dir("atlas-examples-sync", "./examples/sync/data")?;
    let db_path = data_dir.join("db.sqlite");
    tracing::debug!("Data dir: {}", data_dir.display());

    let sync_url = std::env::var(atlas::env::SYNC_URL_KEY).unwrap_or_else(|error| {
        tracing::info!("Couldn't get {}: {}; using fallback.", atlas::env::SYNC_URL_KEY, error);
        tracing::debug!("Hint: set {} to point at a different sqld.", atlas::env::SYNC_URL_KEY);
        DEFAULT_SYNC_URL.to_string()
    });
    let auth_token = std::env::var("LIBSQL_AUTH_TOKEN").unwrap_or_default();

    let database = Builder::new_remote_replica(&db_path, sync_url, auth_token).build().await?;

    print!("Syncing with remote database...");
    database.sync().await?;
    println!(" done");

    let conn = database.connect()?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS guest_book_entries (
            text TEXT NOT NULL
        )",
        (),
    )
    .await?;

    let mut input = String::new();
    println!("Please write your entry to the guestbook:");
    std::io::stdin().read_line(&mut input)?;
    let entry = input.trim();

    if !entry.is_empty() {
        conn.execute("INSERT INTO guest_book_entries (text) VALUES (?1)", params![entry]).await?;
        database.sync().await?;
    }

    let mut results = conn.query("SELECT text FROM guest_book_entries", ()).await?;
    println!("Guest book entries:");
    while let Some(row) = results.next().await? {
        let text: String = row.get(0)?;
        println!("  {text}");
    }

    Ok(())
}
