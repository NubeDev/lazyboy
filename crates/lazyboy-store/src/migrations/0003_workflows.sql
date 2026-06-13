-- Workflows, automation, and the membership model (SCOPE.md "Workflows
-- and automation (build step 6)" and "Feeds, membership, and
-- visibility"). Same id/timestamp/FK conventions as 0001/0002. The
-- membership tables are modeled here but stay OUT of the MVP trust gate
-- under R4 until promoted (see DOCS/WORKFLOWS.md).

CREATE TABLE IF NOT EXISTS workflows (
    id                   TEXT PRIMARY KEY,
    workspace_id         TEXT NOT NULL REFERENCES workspaces(id),
    name                 TEXT NOT NULL,
    trigger_kind         TEXT NOT NULL,                   -- feed | schedule
    trigger_config_json  TEXT,
    approval_policy      TEXT NOT NULL,                   -- require_approval | auto_approve
    steps_json           TEXT NOT NULL,
    status               TEXT NOT NULL,                   -- enabled (== automation) | disabled
    created_at           TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_workflows_workspace ON workflows(workspace_id);

CREATE TABLE IF NOT EXISTS workflow_runs (
    id           TEXT PRIMARY KEY,
    workflow_id  TEXT NOT NULL REFERENCES workflows(id),
    space_id     TEXT NOT NULL REFERENCES spaces(id),
    agent_run_id TEXT NOT NULL REFERENCES agent_runs(id),
    status       TEXT NOT NULL,
    started_at   TEXT NOT NULL,
    ended_at     TEXT
);
CREATE INDEX IF NOT EXISTS idx_workflow_runs_workflow ON workflow_runs(workflow_id);

CREATE TABLE IF NOT EXISTS groups (
    id           TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id),
    name         TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_groups_workspace ON groups(workspace_id);

CREATE TABLE IF NOT EXISTS group_members (
    group_id    TEXT NOT NULL REFERENCES groups(id),
    identity_id TEXT NOT NULL REFERENCES identities(id),
    PRIMARY KEY (group_id, identity_id)
);

CREATE TABLE IF NOT EXISTS space_memberships (
    id             TEXT PRIMARY KEY,
    space_id       TEXT NOT NULL REFERENCES spaces(id),
    principal_kind TEXT NOT NULL,                         -- user | group
    principal_id   TEXT NOT NULL,
    role           TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_space_memberships_space ON space_memberships(space_id);

CREATE TABLE IF NOT EXISTS feed_visibility (
    id                  TEXT PRIMARY KEY,
    feed_integration_id TEXT NOT NULL REFERENCES integrations(id),
    space_id            TEXT NOT NULL REFERENCES spaces(id),
    principal_kind      TEXT NOT NULL,                    -- user | group
    principal_id        TEXT NOT NULL,
    mode                TEXT NOT NULL                     -- visible | hidden
);
CREATE INDEX IF NOT EXISTS idx_feed_visibility_feed ON feed_visibility(feed_integration_id);
