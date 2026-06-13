-- Step-1 timeline tables (SCOPE.md "SQLite domain model"). Only the
-- tables the one-space slice touches are created here; artifacts,
-- decisions, reminders, calendar, integrations, ingress and outbox
-- arrive with their build-order steps rather than as empty stubs.

CREATE TABLE IF NOT EXISTS workspaces (
    id         TEXT PRIMARY KEY,
    name       TEXT NOT NULL,
    created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS spaces (
    id           TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id),
    slug         TEXT NOT NULL,
    title        TEXT NOT NULL,
    status       TEXT NOT NULL,
    created_at   TEXT NOT NULL,
    UNIQUE (workspace_id, slug)
);

CREATE TABLE IF NOT EXISTS identities (
    id           TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id),
    kind         TEXT NOT NULL,
    display_name TEXT NOT NULL,
    external_ref TEXT
);

CREATE TABLE IF NOT EXISTS messages (
    id                 TEXT PRIMARY KEY,
    space_id           TEXT NOT NULL REFERENCES spaces(id),
    author_identity_id TEXT NOT NULL REFERENCES identities(id),
    kind               TEXT NOT NULL,
    body               TEXT NOT NULL,
    ts                 TEXT NOT NULL,
    in_reply_to        TEXT REFERENCES messages(id),
    ref_id             TEXT
);
CREATE INDEX IF NOT EXISTS idx_messages_space_ts ON messages(space_id, ts);

CREATE TABLE IF NOT EXISTS tasks (
    id                     TEXT PRIMARY KEY,
    space_id               TEXT NOT NULL REFERENCES spaces(id),
    title                  TEXT NOT NULL,
    state                  TEXT NOT NULL,
    created_from_message_id TEXT REFERENCES messages(id),
    agent_run_id           TEXT,
    created_at             TEXT NOT NULL,
    updated_at             TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS agent_runs (
    id               TEXT PRIMARY KEY,
    space_id         TEXT NOT NULL REFERENCES spaces(id),
    task_id          TEXT NOT NULL REFERENCES tasks(id),
    goose_session_id TEXT,
    status           TEXT NOT NULL,
    started_at       TEXT,
    ended_at         TEXT
);

CREATE TABLE IF NOT EXISTS agent_run_events (
    id           TEXT PRIMARY KEY,
    agent_run_id TEXT NOT NULL REFERENCES agent_runs(id),
    seq          INTEGER NOT NULL,
    kind         TEXT NOT NULL,
    payload_json TEXT NOT NULL,
    ts           TEXT NOT NULL,
    UNIQUE (agent_run_id, seq)
);

CREATE TABLE IF NOT EXISTS approvals (
    id                     TEXT PRIMARY KEY,
    space_id               TEXT NOT NULL REFERENCES spaces(id),
    agent_run_id           TEXT NOT NULL REFERENCES agent_runs(id),
    goose_session_id       TEXT NOT NULL,
    tool_name              TEXT NOT NULL,
    tool_input_json        TEXT NOT NULL,
    status                 TEXT NOT NULL,
    requested_at           TEXT NOT NULL,
    resolved_at            TEXT,
    resolved_by_identity_id TEXT REFERENCES identities(id)
);
CREATE INDEX IF NOT EXISTS idx_approvals_status ON approvals(status);
