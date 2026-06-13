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
        Ok(())
    }
}
