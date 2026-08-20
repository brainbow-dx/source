//! A minimal local-database example: opens an in-memory `rusqlite` connection, seeds it, and
//! reads it back. Demonstrates `atlas::env::get_data_dir` / `atlas::log::init` — the two helpers
//! every other atlas example builds on — without pulling in the network sync path yet
//! (see `examples/sync` for that).

use anyhow::Result;

use rusqlite::Connection;

struct Person {
    id: i32,
    name: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    atlas::log::init("INFO,atlas_examples_db=TRACE,atlas=TRACE");

    let data_dir = atlas::env::get_data_dir("atlas-examples-db", "./examples/db/data")?;
    tracing::debug!("Data dir: {}", data_dir.display());

    let conn = Connection::open_in_memory()?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS persons (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL
        )",
        (),
    )?;

    conn.execute(
        "INSERT INTO persons (name) VALUES (?1), (?2), (?3)",
        ["Steven", "John", "Alex"],
    )?;

    let mut stmt = conn.prepare("SELECT id, name FROM persons")?;
    let rows = stmt.query_map([], |row| {
        Ok(Person {
            id: row.get(0)?,
            name: row.get(1)?,
        })
    })?;

    tracing::info!("Found persons:");
    for person in rows {
        match person {
            Ok(p) => tracing::info!("ID: {}, Name: {}", p.id, p.name),
            Err(error) => tracing::error!("Error: {error:?}"),
        }
    }

    Ok(())
}
