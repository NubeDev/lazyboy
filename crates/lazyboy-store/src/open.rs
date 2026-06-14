use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};
use sqlx::ConnectOptions;
use std::str::FromStr;

use crate::StoreError;

/// Owns the SQLite pool. One per node (SCOPE.md). Repos borrow the
/// pool through `Store::pool`; they never open their own connections.
#[derive(Clone)]
pub struct Store {
    pool: SqlitePool,
}

impl Store {
    /// Open (creating if absent) the node database at `url`, e.g.
    /// `sqlite://lazyboy.db` or `sqlite::memory:` for tests, and apply
    /// the embedded schema.
    pub async fn connect(url: &str) -> Result<Self, StoreError> {
        let opts = sqlx::sqlite::SqliteConnectOptions::from_str(url)?
            .create_if_missing(true)
            .foreign_keys(true)
            .disable_statement_logging();
        let pool = SqlitePoolOptions::new().connect_with(opts).await?;
        let store = Self { pool };
        store.migrate().await?;
        Ok(store)
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    /// Apply the schema. There is no migration-version table yet, so this
    /// runs on every connect; the schema is written `IF NOT EXISTS` so
    /// reopening a persistent db (the step-1 restart path) is a no-op
    /// rather than a "table already exists" error. A real migration
    /// runner replaces this when a second migration lands.
    async fn migrate(&self) -> Result<(), StoreError> {
        sqlx::query(include_str!("migrations/0001_timeline.sql"))
            .execute(&self.pool)
            .await?;
        sqlx::query(include_str!("migrations/0002_domain.sql"))
            .execute(&self.pool)
            .await?;
        sqlx::query(include_str!("migrations/0003_workflows.sql"))
            .execute(&self.pool)
            .await?;
        self.ensure_runs_task_optional().await?;
        Ok(())
    }

    /// Make `agent_runs.task_id` nullable so a conversation turn can be a
    /// run with no task (chat does not mint a task per message). Guarded by
    /// the current column nullability so the table rebuild runs exactly
    /// once per db, not on every connect. Foreign keys are disabled around
    /// the rebuild because `DROP TABLE` implicitly deletes the rows the
    /// other timeline tables reference; the FK graph is unchanged by the
    /// rebuild (ids are preserved), so it is re-enabled immediately after.
    async fn ensure_runs_task_optional(&self) -> Result<(), StoreError> {
        let mut conn = self.pool.acquire().await?;
        let notnull: i64 = sqlx::query_scalar(
            "SELECT \"notnull\" FROM pragma_table_info('agent_runs') WHERE name = 'task_id'",
        )
        .fetch_one(conn.as_mut())
        .await?;
        if notnull == 0 {
            return Ok(());
        }

        for stmt in [
            "PRAGMA foreign_keys=OFF",
            "BEGIN",
            "CREATE TABLE agent_runs_new (\
                 id TEXT PRIMARY KEY, \
                 space_id TEXT NOT NULL REFERENCES spaces(id), \
                 task_id TEXT REFERENCES tasks(id), \
                 goose_session_id TEXT, \
                 status TEXT NOT NULL, \
                 started_at TEXT, \
                 ended_at TEXT)",
            "INSERT INTO agent_runs_new (id, space_id, task_id, goose_session_id, status, \
                 started_at, ended_at) \
                 SELECT id, space_id, task_id, goose_session_id, status, started_at, ended_at \
                 FROM agent_runs",
            "DROP TABLE agent_runs",
            "ALTER TABLE agent_runs_new RENAME TO agent_runs",
            "COMMIT",
            "PRAGMA foreign_keys=ON",
        ] {
            sqlx::query(stmt).execute(conn.as_mut()).await?;
        }
        Ok(())
    }
}
