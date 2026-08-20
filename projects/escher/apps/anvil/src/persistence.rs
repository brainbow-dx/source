//! sqld/libsql-backed persistence for Anvil's transcript, tasks, per-instance overlay position,
//! and per-instance user settings. Pulled out of `main.rs` on its own — see that file's own
//! `PersistenceWrite`/`AppState::spawn_connect_persistence` for how the app queues writes and
//! drives the single-writer task around this; this module only ever knows "connect, load, save."

use std::time::Duration;

use libsql::Builder;
use libsql::Connection;
use libsql::Database;
use libsql::params;

use crate::ChatMessage;
use crate::TaskRow;

/// The default target — matches `tools/data/compose.yaml`'s `sqld` service (`8081:8080`).
/// `connect`'s `url` parameter overrides this to sync against someone else's `sqld` primary
/// instead (see `Args::connect`'s own doc comment).
const DEFAULT_SQLD_URL: &str = "http://localhost:8081";
/// The connect/sync calls below have no timeout of their own, so a slow or unresponsive
/// `sqld` would otherwise hang here indefinitely instead of failing into the documented
/// in-memory fallback. Was `3` seconds, too short — a fresh embedded
/// replica has to replay the *entire* replication log from scratch on every launch (see
/// `connect_inner`'s own doc comment for why this never reuses a warm local cache), and
/// against this workspace's actual long-lived `sqld` (accumulated ~19,300 log frames from
/// ordinary day-to-day use), a real first sync measured at ~3.5s — just over the old timeout,
/// so it was failing "by a hair" on a perfectly healthy server, not a broken or slow one.
/// Generous enough to comfortably cover a sync an order of magnitude larger than that before
/// this is revisited.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(20);

/// Within a single tool message's `output` lines, a real newline could legitimately appear
/// in any one line — so joining/splitting on `\n` to store `Vec<String>` in one TEXT column
/// would corrupt data with embedded newlines. `\u{1e}` (ASCII Record Separator) is exactly
/// what it's for and won't collide with anything a task's fake output actually contains.
const OUTPUT_SEPARATOR: char = '\u{1e}';

pub struct Persistence {
    /// Kept alongside `connection`, not dropped after `connect_inner`'s initial sync — `sync`
    /// (below) needs it to pull in other peers' writes to the same primary after startup, not
    /// just once.
    database: Database,
    connection: Connection,
}

impl Persistence {
    /// Opens (creating if needed) the local replica, syncs once against `sqld`, and ensures
    /// the schema exists. `url` overrides the default local target — `None` (the common case)
    /// means `DEFAULT_SQLD_URL`; `Some(_)` is `--connect`, syncing against someone else's
    /// `sqld` primary instead (see `Args::connect`'s own doc comment). Returns `Err` if that
    /// target isn't reachable within `CONNECT_TIMEOUT`, including if it never responds at all.
    /// The caller falls back to in-memory-only operation rather than treating that as fatal,
    /// since this is a demo people will often run without `sqld` up.
    pub async fn connect(url: Option<&str>) -> color_eyre::Result<Self> {
        let url = url.unwrap_or(DEFAULT_SQLD_URL).to_string();
        match tokio::time::timeout(CONNECT_TIMEOUT, Self::connect_inner(url.clone())).await {
            Ok(result) => result,
            Err(_) => Err(color_eyre::eyre::eyre!("Timed out connecting to sqld at {url} after {CONNECT_TIMEOUT:?}")),
        }
    }

    async fn connect_inner(url: String) -> color_eyre::Result<Self> {
        // Lives in this session's own pid-keyed directory (see `anvil_session_dir`), not a
        // fixed filename — running `anvil` twice from the same folder
        // should give two instances synced against the same `sqld` primary, each with their
        // own replica, with no flags or separate directories required. The cost is a full
        // re-sync from the primary on every launch rather than reusing a warm local cache
        // across restarts of the same instance — negligible against a small local `sqld`, and
        // a simple, unconditional guarantee beats detecting file-lock contention and only
        // falling back to a private replica when two instances actually collide.
        let replica_path = crate::anvil_session_dir().join("replica.db");
        let database: Database = Builder::new_remote_replica(
            replica_path,
            url,
            String::new(), // no auth configured on the local dev sqld instance
        )
        .build()
        .await?;

        database.sync().await?;

        let connection = database.connect()?;
        let persistence = Persistence { database, connection };
        persistence.ensure_schema().await?;
        Ok(persistence)
    }

    /// Re-pulls whatever's changed on the primary since the last sync (the initial one in
    /// `connect_inner`, or a previous call to this) into the local replica file — without
    /// this, a second instance pointed at the same primary (e.g. via `--connect`) never sees
    /// writes made after it started, since `connect_inner` only ever syncs once. The caller
    /// (`AppState::spawn_periodic_resync`) is what decides *when* to call this and what to do
    /// with newly-visible rows; this is just "pull," nothing more.
    pub async fn sync(&self) -> color_eyre::Result<()> {
        self.database.sync().await?;
        Ok(())
    }

    async fn ensure_schema(&self) -> color_eyre::Result<()> {
        // `overlay_state` used to be a single global row (`id INTEGER PRIMARY KEY CHECK (id =
        // 1)`) shared by every instance connected to the same `sqld` — that's wrong: the
        // transcript/task stream should stay shared across everyone, but
        // *where your overlay window sits* is per-instance, per-person, and one person
        // dragging theirs was silently relocating everyone else's on their next resync. Keyed
        // by `identity` (`Args::identity`) instead now. SQLite can't just `ALTER TABLE` a
        // primary key, so an install that predates this re-keys once: `identity` missing means
        // either the old single-row shape or a brand-new database, both of which are safe to
        // drop and recreate — a saved window position is the only thing that could be lost,
        // not worth a real migration for.
        let has_identity_column = {
            let mut rows = self.connection.query("SELECT 1 FROM pragma_table_info('overlay_state') WHERE name = 'identity'", ()).await?;
            rows.next().await?.is_some()
        };
        if !has_identity_column {
            self.connection.execute_batch("DROP TABLE IF EXISTS overlay_state;").await?;
        }

        self.connection
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS messages (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    kind TEXT NOT NULL,
                    content TEXT,
                    tool_name TEXT,
                    tool_detail TEXT,
                    tool_output TEXT,
                    hidden INTEGER NOT NULL DEFAULT 0,
                    created_at INTEGER NOT NULL
                );
                CREATE TABLE IF NOT EXISTS tasks (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    label TEXT NOT NULL,
                    status TEXT NOT NULL,
                    created_at INTEGER NOT NULL
                );
                CREATE TABLE IF NOT EXISTS overlay_state (
                    identity BLOB PRIMARY KEY,
                    x INTEGER NOT NULL,
                    y INTEGER NOT NULL,
                    width INTEGER NOT NULL,
                    height INTEGER NOT NULL,
                    updated_at INTEGER NOT NULL
                );
                CREATE TABLE IF NOT EXISTS user_settings (
                    identity BLOB PRIMARY KEY,
                    show_welcome_overview INTEGER NOT NULL DEFAULT 1
                );",
            )
            .await?;

        // `messages` may already exist from before `hidden` was added — `CREATE TABLE IF NOT
        // EXISTS` above is a no-op against an existing table, and SQLite has no `ADD COLUMN IF
        // NOT EXISTS`. Allowed to fail (already applied) rather than `?`'d — same "run the
        // migration every startup, tolerate it already being there" approach the `CREATE
        // TABLE`s above already take.
        let _ = self.connection.execute("ALTER TABLE messages ADD COLUMN hidden INTEGER NOT NULL DEFAULT 0", ()).await;
        // Same idea, for `user_settings` gaining the sidebar's own remembered width — `220.0`
        // matches `escher_appkit::bevy::TabStripState::default`'s own starting width, so an
        // install migrating from before this existed sees no visible change until it actually
        // resizes/collapses the sidebar for the first time.
        let _ = self.connection.execute("ALTER TABLE user_settings ADD COLUMN sidebar_width REAL NOT NULL DEFAULT 220.0", ()).await;
        let _ = self.connection.execute("ALTER TABLE user_settings ADD COLUMN sidebar_expanded_width REAL NOT NULL DEFAULT 220.0", ()).await;

        Ok(())
    }

    /// Wipes every message and task — used by `--reset-data`. Real deletes against the
    /// remote `sqld` primary (not just the local replica file), so this actually clears the
    /// data everyone connecting to this `sqld` instance sees, not just this machine's cache.
    pub async fn reset(&self) -> color_eyre::Result<()> {
        self.connection
            .execute_batch("DELETE FROM messages; DELETE FROM tasks; DELETE FROM overlay_state;")
            .await?;
        Ok(())
    }

    pub async fn load_messages(&self) -> color_eyre::Result<Vec<ChatMessage>> {
        let mut rows = self
            .connection
            .query(
                "SELECT kind, content, tool_name, tool_detail, tool_output FROM messages WHERE hidden = 0 ORDER BY id ASC",
                (),
            )
            .await?;

        let mut messages = Vec::new();

        while let Some(row) = rows.next().await? {
            let kind: String = row.get(0)?;
            match kind.as_str() {
                "user" => messages.push(ChatMessage::User(row.get(1)?)),
                "assistant" => messages.push(ChatMessage::Assistant(row.get(1)?)),
                "tool" => {
                    let output_joined: String = row.get::<Option<String>>(4)?.unwrap_or_default();
                    let output = if output_joined.is_empty() {
                        Vec::new()
                    } else {
                        output_joined.split(OUTPUT_SEPARATOR).map(String::from).collect()
                    };

                    messages.push(ChatMessage::Tool {
                        name: row.get(2)?,
                        detail: row.get(3)?,
                        output,
                    });
                }
                other => tracing::warn!("Unknown message kind {other:?} in database, skipping"),
            }
        }

        Ok(messages)
    }

    /// Whether `message` is worth recording for tracing/audit but never worth showing in the
    /// live transcript on reload — recorded, but not everything logged needs to show up in the
    /// feed again. `/quit`'s own request/tool-call/reply trio is the one case so far — every
    /// future session otherwise reloads to a stale "someone quit" note nobody needs to see
    /// again. Checked once here, at the one place every message flows through before being
    /// written, via a real `hidden` column and a `WHERE hidden = 0` on `load_messages`'s own
    /// query — filtering belongs in the SQL, not as an app-side post-filter — rather than
    /// needing every call site that might produce one of these three to remember to flag it
    /// itself.
    fn is_hidden_from_history(message: &ChatMessage) -> bool {
        match message {
            ChatMessage::User(text) => text.trim() == "/quit",
            ChatMessage::Tool { name, detail, .. } => name == "js" && detail == "quit",
            ChatMessage::Assistant(text) => text.trim() == crate::QUIT_SENTINEL,
            ChatMessage::Trace(_) => false,
        }
    }

    pub async fn save_message(&self, message: &ChatMessage) -> color_eyre::Result<()> {
        let hidden = Self::is_hidden_from_history(message);
        match message {
            ChatMessage::User(text) => {
                self.connection
                    .execute(
                        "INSERT INTO messages (kind, content, hidden, created_at) VALUES ('user', ?1, ?2, ?3)",
                        params![text.as_str(), hidden, now_millis()],
                    )
                    .await?;
            }
            ChatMessage::Assistant(text) => {
                self.connection
                    .execute(
                        "INSERT INTO messages (kind, content, hidden, created_at) VALUES ('assistant', ?1, ?2, ?3)",
                        params![text.as_str(), hidden, now_millis()],
                    )
                    .await?;
            }
            ChatMessage::Tool { name, detail, output } => {
                let joined = output.join(&OUTPUT_SEPARATOR.to_string());
                self.connection
                    .execute(
                        "INSERT INTO messages (kind, tool_name, tool_detail, tool_output, hidden, created_at) \
                         VALUES ('tool', ?1, ?2, ?3, ?4, ?5)",
                        params![name.as_str(), detail.as_str(), joined, hidden, now_millis()],
                    )
                    .await?;
            }
            // Ephemeral by design, see `ChatMessage::Trace`'s own doc comment.
            ChatMessage::Trace(_) => {}
        }
        Ok(())
    }

    pub async fn load_tasks(&self) -> color_eyre::Result<Vec<TaskRow>> {
        let mut rows = self
            .connection
            .query("SELECT label, status FROM tasks ORDER BY id ASC", ())
            .await?;

        let mut tasks = Vec::new();
        while let Some(row) = rows.next().await? {
            tasks.push(TaskRow { label: row.get(0)?, status: row.get(1)? });
        }
        Ok(tasks)
    }

    /// Replaces the whole `tasks` table with exactly `tasks`, in order — not a per-row
    /// upsert. `TaskRow` carries no id a caller could target for an `UPDATE`, and the table
    /// has no unique key on `label` (two tasks can legitimately share a label), so there's no
    /// safe way to address "the row that changed" from here. The in-memory `Vec<TaskRow>` is
    /// the source of truth; this just makes the table match it, the same way `save_tasks`'s
    /// caller already keeps the UI in sync with that same `Vec`.
    ///
    /// Wrapped in a real transaction, not run as loose statements — every call site goes
    /// through `with_sqld_timeout` (750ms), and without a transaction a timeout firing
    /// between the `DELETE` committing and the `INSERT` loop finishing would leave the table
    /// permanently short some tasks (the delete already landed; the re-inserts didn't).
    /// `libsql::Transaction` rolls back automatically if it's dropped without `commit()` —
    /// exactly what happens when `tokio::time::timeout` drops this future mid-flight — so a
    /// timeout now leaves the previous, still-correct table in place instead of a half-empty
    /// one.
    pub async fn save_tasks(&self, tasks: &[TaskRow]) -> color_eyre::Result<()> {
        let tx = self.connection.transaction().await?;
        tx.execute("DELETE FROM tasks", ()).await?;
        for task in tasks {
            tx.execute(
                "INSERT INTO tasks (label, status, created_at) VALUES (?1, ?2, ?3)",
                params![task.label.as_str(), task.status.as_str(), now_millis()],
            )
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    /// `(x, y, width, height)` in plain `u16`s rather than `ratatui::layout::Rect` — this
    /// module deliberately doesn't depend on `ratatui` (see `ChatMessage`/`TaskRow`, defined
    /// outside it for the same reason), the caller reassembles the `Rect` itself. Keyed by
    /// `identity_uuid` (see `ANVIL_IDENTITY_NAMESPACE`'s doc comment) — see `ensure_schema`'s
    /// own doc comment for why this is per-identity, not one shared row.
    pub async fn load_overlay_bounds(&self, identity_uuid: &uuid::Uuid) -> color_eyre::Result<Option<(u16, u16, u16, u16)>> {
        let mut rows = self
            .connection
            .query("SELECT x, y, width, height FROM overlay_state WHERE identity = ?1", params![identity_uuid.as_bytes().to_vec()])
            .await?;

        match rows.next().await? {
            Some(row) => {
                let x: i64 = row.get(0)?;
                let y: i64 = row.get(1)?;
                let width: i64 = row.get(2)?;
                let height: i64 = row.get(3)?;
                Ok(Some((x as u16, y as u16, width as u16, height as u16)))
            }
            None => Ok(None),
        }
    }

    /// A per-`identity_uuid` upsert — one row per instance/person, not one shared row (see
    /// `ensure_schema`'s own doc comment for why this changed from a single global row).
    pub async fn save_overlay_bounds(&self, identity_uuid: &uuid::Uuid, bounds: (u16, u16, u16, u16)) -> color_eyre::Result<()> {
        let (x, y, width, height) = bounds;
        self.connection
            .execute(
                "INSERT INTO overlay_state (identity, x, y, width, height, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6) \
                 ON CONFLICT (identity) DO UPDATE SET x = ?2, y = ?3, width = ?4, height = ?5, updated_at = ?6",
                params![identity_uuid.as_bytes().to_vec(), x as i64, y as i64, width as i64, height as i64, now_millis()],
            )
            .await?;
        Ok(())
    }

    /// Defaults to `true` (shown) for an identity with no row yet — a brand new user, or one
    /// synced from before this setting existed, sees the overview at least once.
    pub async fn load_show_welcome_overview(&self, identity_uuid: &uuid::Uuid) -> color_eyre::Result<bool> {
        let mut rows = self
            .connection
            .query("SELECT show_welcome_overview FROM user_settings WHERE identity = ?1", params![identity_uuid.as_bytes().to_vec()])
            .await?;

        match rows.next().await? {
            Some(row) => Ok(row.get::<i64>(0)? != 0),
            None => Ok(true),
        }
    }

    pub async fn save_show_welcome_overview(&self, identity_uuid: &uuid::Uuid, show: bool) -> color_eyre::Result<()> {
        self.connection
            .execute(
                "INSERT INTO user_settings (identity, show_welcome_overview) VALUES (?1, ?2) \
                 ON CONFLICT (identity) DO UPDATE SET show_welcome_overview = ?2",
                params![identity_uuid.as_bytes().to_vec(), show as i64],
            )
            .await?;
        Ok(())
    }

    /// `(width, expanded_width)` — see `escher_appkit::bevy::TabStripState`'s own doc comment for
    /// why both, not just `width`, need remembering. `220.0`/`220.0` (that struct's own `Default`)
    /// for an identity with no row yet, same "brand new user sees the untouched default" contract
    /// `load_show_welcome_overview` already follows.
    pub async fn load_sidebar_state(&self, identity_uuid: &uuid::Uuid) -> color_eyre::Result<(f64, f64)> {
        let mut rows = self
            .connection
            .query("SELECT sidebar_width, sidebar_expanded_width FROM user_settings WHERE identity = ?1", params![identity_uuid.as_bytes().to_vec()])
            .await?;

        match rows.next().await? {
            Some(row) => Ok((row.get(0)?, row.get(1)?)),
            None => Ok((220.0, 220.0)),
        }
    }

    pub async fn save_sidebar_state(&self, identity_uuid: &uuid::Uuid, width: f64, expanded_width: f64) -> color_eyre::Result<()> {
        self.connection
            .execute(
                "INSERT INTO user_settings (identity, sidebar_width, sidebar_expanded_width) VALUES (?1, ?2, ?3) \
                 ON CONFLICT (identity) DO UPDATE SET sidebar_width = ?2, sidebar_expanded_width = ?3",
                params![identity_uuid.as_bytes().to_vec(), width, expanded_width],
            )
            .await?;
        Ok(())
    }
}

fn now_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}
