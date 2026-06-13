-- Domain tables beyond the step-1 timeline slice (SCOPE.md "SQLite domain
-- model"): artifacts, decisions, reminders, calendar, integrations, ingress
-- and outbox. Same id/timestamp/FK conventions as 0001_timeline.sql.

CREATE TABLE IF NOT EXISTS artifacts (
    id           TEXT PRIMARY KEY,
    space_id     TEXT NOT NULL REFERENCES spaces(id),
    agent_run_id TEXT REFERENCES agent_runs(id),
    kind         TEXT NOT NULL,
    uri          TEXT NOT NULL,
    meta_json    TEXT,
    created_at   TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_artifacts_space ON artifacts(space_id);

CREATE TABLE IF NOT EXISTS decisions (
    id                    TEXT PRIMARY KEY,
    space_id              TEXT NOT NULL REFERENCES spaces(id),
    message_id            TEXT REFERENCES messages(id),
    summary               TEXT NOT NULL,
    decided_by_identity_id TEXT REFERENCES identities(id),
    decided_at            TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_decisions_space ON decisions(space_id);

CREATE TABLE IF NOT EXISTS reminders (
    id       TEXT PRIMARY KEY,
    space_id TEXT NOT NULL REFERENCES spaces(id),
    task_id  TEXT REFERENCES tasks(id),
    due_at   TEXT NOT NULL,
    body     TEXT NOT NULL,
    status   TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_reminders_due ON reminders(due_at);

CREATE TABLE IF NOT EXISTS calendar_events (
    id           TEXT PRIMARY KEY,
    space_id     TEXT NOT NULL REFERENCES spaces(id),
    source       TEXT NOT NULL,
    external_ref TEXT,
    title        TEXT NOT NULL,
    starts_at    TEXT NOT NULL,
    ends_at      TEXT,
    meta_json    TEXT
);
CREATE INDEX IF NOT EXISTS idx_calendar_events_space_starts ON calendar_events(space_id, starts_at);

CREATE TABLE IF NOT EXISTS integrations (
    id           TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id),
    provider     TEXT NOT NULL,
    account_ref  TEXT,
    secret_ref   TEXT,
    status       TEXT NOT NULL,
    config_json  TEXT
);
CREATE INDEX IF NOT EXISTS idx_integrations_workspace ON integrations(workspace_id);

CREATE TABLE IF NOT EXISTS ingress_events (
    id             TEXT PRIMARY KEY,
    integration_id TEXT NOT NULL REFERENCES integrations(id),
    space_id       TEXT NOT NULL REFERENCES spaces(id),
    external_id    TEXT NOT NULL,
    kind           TEXT NOT NULL,
    payload_json   TEXT NOT NULL,
    message_id     TEXT REFERENCES messages(id),
    received_at    TEXT NOT NULL,
    UNIQUE (integration_id, external_id)
);

CREATE TABLE IF NOT EXISTS outbox_events (
    id           TEXT PRIMARY KEY,
    aggregate    TEXT NOT NULL,
    aggregate_id TEXT NOT NULL,
    event_json   TEXT NOT NULL,
    seq          INTEGER NOT NULL,
    created_at   TEXT NOT NULL,
    synced_at    TEXT,
    UNIQUE (aggregate, seq)
);
CREATE INDEX IF NOT EXISTS idx_outbox_unsynced_seq ON outbox_events(synced_at, seq);
