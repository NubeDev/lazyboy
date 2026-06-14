//! Guards the one-time rebuild that makes `agent_runs.task_id` nullable on
//! an existing database. The chat-turn model depends on a run with no
//! task; a db created before that change has the column `NOT NULL`, and
//! `Store::connect` must migrate it in place without losing data or
//! tripping foreign keys.

use std::str::FromStr;

use sqlx::sqlite::{SqliteConnectOptions, SqlitePool};
use sqlx::Row;

use lazyboy_store::Store;

/// Build an old-schema db on disk (with `task_id NOT NULL` and a row that
/// references a task), then open it through `Store::connect` and confirm
/// the column became nullable, the existing row survived, and a new
/// task-less run can be inserted.
#[tokio::test]
async fn migrates_existing_not_null_task_id_to_nullable() {
    let path = std::env::temp_dir().join(format!("lazyboy-mig-{}.db", std::process::id()));
    let _ = std::fs::remove_file(&path);
    let url = format!("sqlite://{}", path.display());

    // Pre-seed the pre-change schema: agent_runs.task_id is NOT NULL, with
    // one run pointing at a real task. The other timeline tables are left
    // for `Store::connect`'s `IF NOT EXISTS` migrations to create.
    {
        let opts = SqliteConnectOptions::from_str(&url)
            .unwrap()
            .create_if_missing(true);
        let pool = SqlitePool::connect_with(opts).await.unwrap();
        for stmt in [
            "CREATE TABLE workspaces (id TEXT PRIMARY KEY, name TEXT NOT NULL, created_at TEXT NOT NULL)",
            "CREATE TABLE spaces (id TEXT PRIMARY KEY, workspace_id TEXT NOT NULL, slug TEXT NOT NULL, title TEXT NOT NULL, status TEXT NOT NULL, created_at TEXT NOT NULL)",
            "CREATE TABLE tasks (id TEXT PRIMARY KEY, space_id TEXT NOT NULL, title TEXT NOT NULL, state TEXT NOT NULL, created_from_message_id TEXT, agent_run_id TEXT, created_at TEXT NOT NULL, updated_at TEXT NOT NULL)",
            "CREATE TABLE agent_runs (id TEXT PRIMARY KEY, space_id TEXT NOT NULL REFERENCES spaces(id), task_id TEXT NOT NULL REFERENCES tasks(id), goose_session_id TEXT, status TEXT NOT NULL, started_at TEXT, ended_at TEXT)",
            "INSERT INTO workspaces VALUES ('w','acme','t')",
            "INSERT INTO spaces VALUES ('s','w','home','Home','active','t')",
            "INSERT INTO tasks VALUES ('tk','s','ship','open',NULL,NULL,'t','t')",
            "INSERT INTO agent_runs VALUES ('r','s','tk',NULL,'succeeded','t',NULL)",
        ] {
            sqlx::query(stmt).execute(&pool).await.unwrap();
        }
        pool.close().await;
    }

    let store = Store::connect(&url).await.unwrap();

    // The column is now nullable.
    let notnull: i64 = sqlx::query_scalar(
        "SELECT \"notnull\" FROM pragma_table_info('agent_runs') WHERE name = 'task_id'",
    )
    .fetch_one(store.pool())
    .await
    .unwrap();
    assert_eq!(notnull, 0, "task_id must be nullable after migration");

    // The existing task-backed run survived the rebuild.
    let row = sqlx::query("SELECT task_id FROM agent_runs WHERE id = 'r'")
        .fetch_one(store.pool())
        .await
        .unwrap();
    assert_eq!(row.get::<Option<String>, _>("task_id"), Some("tk".to_owned()));

    // A new chat run with no task now inserts.
    sqlx::query(
        "INSERT INTO agent_runs (id, space_id, task_id, goose_session_id, status, started_at, ended_at) \
         VALUES ('r2','s',NULL,NULL,'queued','t',NULL)",
    )
    .execute(store.pool())
    .await
    .unwrap();

    // Foreign keys still hold and the file is consistent.
    let fk_violations = sqlx::query("PRAGMA foreign_key_check")
        .fetch_all(store.pool())
        .await
        .unwrap();
    assert!(fk_violations.is_empty(), "no foreign key violations after migration");

    let _ = std::fs::remove_file(&path);
}
