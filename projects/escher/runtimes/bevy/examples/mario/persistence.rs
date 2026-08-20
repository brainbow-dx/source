//! Persists two things across restarts and across machines pointed at the same `sqld`: which
//! gamepad a machine currently sees (for ownership arbitration between instances on the same
//! host) and every lost life, ever (the permanent ghosts).
//!
//! Backed by a real `sqld` via an embedded `libsql` replica, the same transport Anvil's own chat
//! and task persistence uses. This is expected to move onto Atlas's own store abstraction once
//! that grows real persistence support beyond its current in-memory implementation, rather than
//! staying a bespoke `sqld` client per app. See Escher's `spec/ROADMAP.md`.

use std::time::Duration;

use libsql::params;
use libsql::Builder;
use libsql::Connection;
use libsql::Database;

const DEFAULT_SQLD_URL: &str = "http://localhost:8081";
/// The connect/sync calls below have no timeout of their own, so a slow or unresponsive `sqld`
/// would otherwise hang here indefinitely instead of falling back to running without persistence.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(20);
/// How long a single persistence write is allowed to block before giving up, so a slow or wedged
/// `sqld` can't stall the render loop.
const SAVE_TIMEOUT: Duration = Duration::from_millis(750);

pub struct Persistence {
    /// Kept alongside `connection`, not dropped after connecting: `sync` needs it to pull in other
    /// peers' writes to the same primary after startup, not just once.
    database: Database,
    connection: Connection,
}

impl Persistence {
    /// Opens (creating if needed) a local replica keyed to this process, syncs once against
    /// `sqld`, and ensures the schema exists. Returns `Err` if `sqld` isn't reachable within
    /// `CONNECT_TIMEOUT`. The caller falls back to running without persistence rather than
    /// treating that as fatal.
    pub async fn connect(url: Option<&str>) -> color_eyre::Result<Self> {
        let url = url.unwrap_or(DEFAULT_SQLD_URL).to_string();
        match tokio::time::timeout(CONNECT_TIMEOUT, Self::connect_inner(url.clone())).await {
            Ok(result) => result,
            Err(_) => Err(color_eyre::eyre::eyre!("Timed out connecting to sqld at {url} after {CONNECT_TIMEOUT:?}")),
        }
    }

    async fn connect_inner(url: String) -> color_eyre::Result<Self> {
        let replica_path = std::env::temp_dir().join(format!("escher-mario-{}", std::process::id())).join("replica.db");
        if let Some(parent) = replica_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let database: Database = Builder::new_remote_replica(replica_path, url, String::new()).build().await?;
        database.sync().await?;

        let connection = database.connect()?;
        let persistence = Persistence { database, connection };
        persistence.ensure_schema().await?;
        Ok(persistence)
    }

    /// Re-pulls whatever changed on the primary since the last sync into the local replica file.
    pub async fn sync(&self) -> color_eyre::Result<()> {
        self.database.sync().await?;
        Ok(())
    }

    async fn ensure_schema(&self) -> color_eyre::Result<()> {
        self.connection
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS gamepad_sightings (
                    candidate_id TEXT NOT NULL,
                    identity BLOB NOT NULL,
                    identity_name TEXT NOT NULL DEFAULT '',
                    last_seen INTEGER NOT NULL,
                    PRIMARY KEY (candidate_id, identity)
                );
                CREATE TABLE IF NOT EXISTS defeated_players (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    candidate_id TEXT NOT NULL,
                    name TEXT NOT NULL,
                    color_r INTEGER NOT NULL,
                    color_g INTEGER NOT NULL,
                    color_b INTEGER NOT NULL,
                    defeated_at INTEGER NOT NULL
                );",
            )
            .await?;
        Ok(())
    }

    /// Refreshes this instance's own "I currently see these gamepads" rows. One row per
    /// `candidate_id` per `identity_uuid`. A candidate this instance no longer sees is deleted
    /// outright rather than left to age out, so unplugging a controller stops its sighting
    /// immediately rather than eventually.
    pub async fn save_gamepad_sightings(&self, identity_uuid: &uuid::Uuid, identity_name: &str, candidate_ids: &[String]) -> color_eyre::Result<()> {
        let identity_bytes = identity_uuid.as_bytes().to_vec();
        let now = now_millis();

        self.connection.execute("DELETE FROM gamepad_sightings WHERE identity = ?1", params![identity_bytes.clone()]).await?;
        for candidate_id in candidate_ids {
            self.connection
                .execute(
                    "INSERT INTO gamepad_sightings (candidate_id, identity, identity_name, last_seen) VALUES (?1, ?2, ?3, ?4)",
                    params![candidate_id.as_str(), identity_bytes.clone(), identity_name, now],
                )
                .await?;
        }
        Ok(())
    }

    /// Every instance's current gamepad sightings: `(candidate_id, identity_uuid, identity_name,
    /// last_seen)`, the raw material ownership arbitration reduces into who owns what and the
    /// connected-players roster.
    pub async fn load_gamepad_sightings(&self) -> color_eyre::Result<Vec<(String, uuid::Uuid, String, i64)>> {
        let mut rows = self.connection.query("SELECT candidate_id, identity, identity_name, last_seen FROM gamepad_sightings", ()).await?;

        let mut sightings = Vec::new();
        while let Some(row) = rows.next().await? {
            let candidate_id: String = row.get(0)?;
            let identity_bytes: Vec<u8> = row.get(1)?;
            let identity_name: String = row.get(2)?;
            let last_seen: i64 = row.get(3)?;
            let Ok(identity_bytes) = <[u8; 16]>::try_from(identity_bytes.as_slice()) else {
                tracing::warn!("Skipping gamepad sighting with malformed identity for {candidate_id:?}");
                continue;
            };
            sightings.push((candidate_id, uuid::Uuid::from_bytes(identity_bytes), identity_name, last_seen));
        }
        Ok(sightings)
    }

    /// Records one lost life as a permanent row. Every lost life becomes a ghost, not just the one
    /// that exhausts a player's lives, so this always appends rather than upserting over a
    /// previous row for the same player. `color` is baked in at the moment of death rather than
    /// re-resolved later, since a ghost keeps the color it actually died with even if a future
    /// session assigns a different color to the same candidate id.
    pub async fn save_defeat(&self, candidate_id: &str, name: &str, color: (u8, u8, u8)) -> color_eyre::Result<()> {
        self.connection
            .execute(
                "INSERT INTO defeated_players (candidate_id, name, color_r, color_g, color_b, defeated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![candidate_id, name, color.0 as i64, color.1 as i64, color.2 as i64, now_millis()],
            )
            .await?;
        Ok(())
    }

    /// Every lost life ever recorded, oldest first.
    pub async fn load_ghosts(&self) -> color_eyre::Result<Vec<(String, String, (u8, u8, u8), i64)>> {
        let mut rows =
            self.connection.query("SELECT candidate_id, name, color_r, color_g, color_b, defeated_at FROM defeated_players ORDER BY defeated_at ASC", ()).await?;

        let mut ghosts = Vec::new();
        while let Some(row) = rows.next().await? {
            let candidate_id: String = row.get(0)?;
            let name: String = row.get(1)?;
            let color_r: i64 = row.get(2)?;
            let color_g: i64 = row.get(3)?;
            let color_b: i64 = row.get(4)?;
            let defeated_at: i64 = row.get(5)?;
            ghosts.push((candidate_id, name, (color_r as u8, color_g as u8, color_b as u8), defeated_at));
        }
        Ok(ghosts)
    }
}

fn now_millis() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis() as i64
}

/// Runs `future`, a single `Persistence` write, bounded by `SAVE_TIMEOUT`. Flattens "the operation
/// failed" and "it timed out" into one `Result<T, String>`, since every call site only ever warns
/// either way.
async fn save_with_timeout<F, T>(future: F) -> Result<T, String>
where
    F: std::future::Future<Output = color_eyre::Result<T>>,
{
    match tokio::time::timeout(SAVE_TIMEOUT, future).await {
        Ok(Ok(value)) => Ok(value),
        Ok(Err(error)) => Err(error.to_string()),
        Err(_) => Err(format!("timed out after {SAVE_TIMEOUT:?} (sqld slow or unresponsive)")),
    }
}

/// One pending persistence write, deferred until the single writer task's turn instead of racing
/// straight into the shared connection from wherever the write originated.
pub enum PersistenceWrite {
    Defeat { candidate_id: String, name: String, color: (u8, u8, u8) },
}

/// Connects to `sqld` in the background, loads existing gamepad sightings and ghosts, and starts
/// the ownership-reconciliation loop and the single persistence-writer task. Every write goes
/// through the one writer task, in order, so nothing else ever touches the connection directly.
/// Failures anywhere in here are logged and swallowed: local, single-player play keeps working with
/// no `sqld` reachable at all.
pub fn spawn_connect_persistence(state: crate::GameState) {
    let outer_runtime = state.runtime.clone();
    outer_runtime.spawn(async move {
        let store = match Persistence::connect(std::env::var(atlas::env::SYNC_URL_KEY).ok().as_deref()).await {
            Ok(store) => store,
            Err(error) => {
                tracing::warn!("Could not connect to sqld, running without persistence: {error}");
                return;
            }
        };
        let store = std::sync::Arc::new(store);
        *state.persistence.write() = Some(store.clone());

        const RECONCILE_INTERVAL: Duration = Duration::from_secs(2);
        // A sighting older than this doesn't count as "currently seeing it", comfortably longer
        // than `RECONCILE_INTERVAL` so a couple of missed rounds don't spuriously free up a
        // candidate still actually in use.
        const FRESHNESS_WINDOW_MILLIS: i64 = 6_000;

        {
            let store = store.clone();
            let state = state.clone();
            state.runtime.spawn(async move {
                loop {
                    tokio::time::sleep(RECONCILE_INTERVAL).await;

                    if let Err(error) = store.sync().await {
                        tracing::warn!("Periodic sqld resync failed: {error}");
                        continue;
                    }

                    let candidates = state.visible_gamepad_candidates.read().clone();
                    if let Err(error) = store.save_gamepad_sightings(&state.identity_uuid, &state.identity, &candidates).await {
                        tracing::warn!("Failed to publish gamepad sightings to sqld: {error}");
                        continue;
                    }

                    let sightings = match store.load_gamepad_sightings().await {
                        Ok(sightings) => sightings,
                        Err(error) => {
                            tracing::warn!("Failed to load gamepad sightings from sqld: {error}");
                            continue;
                        }
                    };

                    let now = now_millis();
                    let is_fresh = |last_seen: &i64| now - last_seen < FRESHNESS_WINDOW_MILLIS;

                    let owned = candidates
                        .into_iter()
                        .filter(|candidate_id| {
                            let owner = sightings
                                .iter()
                                .filter(|(id, _, _, last_seen)| id == candidate_id && is_fresh(last_seen))
                                .map(|(_, identity, ..)| *identity)
                                .min();
                            owner == Some(state.identity_uuid)
                        })
                        .collect();
                    *state.gamepad_owned_by_me.write() = owned;

                    // The connected-players roster: every candidate anyone currently sees, one
                    // entry each, labeled and colored by its resolved owner. Every instance
                    // computes the identical roster independently from the same synced data.
                    let mut owners: Vec<(String, uuid::Uuid, String)> = Vec::new();
                    for candidate_id in sightings.iter().map(|(id, ..)| id).collect::<std::collections::HashSet<_>>() {
                        let owner = sightings.iter().filter(|(id, _, _, last_seen)| id == candidate_id && is_fresh(last_seen)).min_by_key(|(_, identity, ..)| *identity);
                        if let Some((_, owner_identity, owner_name, _)) = owner {
                            owners.push((candidate_id.clone(), *owner_identity, owner_name.clone()));
                        }
                    }
                    owners.sort_by(|a, b| (a.1, &a.0).cmp(&(b.1, &b.0)));
                    *state.connected_players.write() = owners
                        .into_iter()
                        .enumerate()
                        .map(|(index, (candidate_id, _, name))| (candidate_id, name, crate::physics::mario_player_color(index)))
                        .collect();

                    match store.load_ghosts().await {
                        Ok(loaded) => {
                            *state.ghosts.write() = loaded
                                .into_iter()
                                .map(|(candidate_id, name, color, defeated_at)| {
                                    let drift = crate::ghosts::mario_ghost_drift(&candidate_id, defeated_at);
                                    crate::ghosts::GhostEntry { candidate_id, name, color, drift }
                                })
                                .collect();
                        }
                        Err(error) => tracing::warn!("Failed to load ghosts from sqld: {error}"),
                    }
                }
            });
        }

        let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel::<PersistenceWrite>();
        *state.persistence_writes.write() = Some(sender);

        state.runtime.spawn(async move {
            while let Some(write) = receiver.recv().await {
                match write {
                    PersistenceWrite::Defeat { candidate_id, name, color } => {
                        if let Err(error) = save_with_timeout(store.save_defeat(&candidate_id, &name, color)).await {
                            tracing::warn!("Failed to persist a defeat to sqld: {error}");
                        }
                    }
                }
            }
        });
    });
}
