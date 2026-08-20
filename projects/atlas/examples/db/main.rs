//! A minimal local-database example: opens an in-memory `libsql` connection, seeds it, and reads
//! it back. Demonstrates `atlas::env::get_data_dir` / `atlas::log::init` — the two helpers every
//! other atlas example builds on — without pulling in the network sync path yet (see
//! `examples/sync` for that, which layers a remote-replica `libsql` connection on top of the same
//! two helpers). `libsql`, not `rusqlite`, since that's the sqlite binding this project actually
//! uses everywhere else — nothing here needs a second one.

use anyhow::Result;

use libsql::Builder;
use libsql::params;

struct Person {
    id: i64,
    name: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    atlas::log::init("INFO,atlas_examples_db=TRACE,atlas=TRACE");

    let data_dir = atlas::env::get_data_dir("atlas-examples-db", "./examples/db/data")?;
    tracing::debug!("Data dir: {}", data_dir.display());

    let database = Builder::new_local(":memory:").build().await?;
    let conn = database.connect()?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS persons (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL
        )",
        (),
    )
    .await?;

    conn.execute(
        "INSERT INTO persons (name) VALUES (?1), (?2), (?3)",
        params!["Steven", "John", "Alex"],
    )
    .await?;

    let mut rows = conn.query("SELECT id, name FROM persons", ()).await?;

    tracing::info!("Found persons:");
    while let Some(row) = rows.next().await? {
        let person = Person { id: row.get(0)?, name: row.get(1)? };
        tracing::info!("ID: {}, Name: {}", person.id, person.name);
    }

    Ok(())
}
