# Lazyboy IDEA-V2

This document replaces `IDEA.md` with a narrower, code-grounded scope for the
first real version of Lazyboy.

It is based on direct inspection of the two upstream repos named by the idea:

- Lukan: `https://github.com/lukanlabs/lukan`
  - local clone: `/tmp/lazyboy-review.DQvcU0/lukan`
  - inspected commit: `2008026c7e8789d548d057d5e1ea651907ebcad1`
  - commit date: 2026-06-07
  - observed version: `0.1.32`
- ZeroClaw: `https://github.com/zeroclaw-labs/zeroclaw`
  - local clone: `/tmp/lazyboy-review.DQvcU0/zeroclaw`
  - inspected commit: `ffb027554c86be359a80d29fb3a46e1d6d2950d3`
  - commit date: 2026-06-11
  - observed version: `0.8.0`

Zenoh was not cloned in this pass. The request was to inspect both repos, and
the two product repos are Lukan and ZeroClaw. Zenoh remains a later sync
candidate, not a first-version dependency.

## Verdict

The original idea has a strong product center:

```text
Channels should become the work surface for humans and agents.
```

That is worth keeping.

The original implementation scope is too large:

```text
Slack + Asana + email + workflow engine + desktop app + agent runtime +
sync layer + Windows port + cloud deployment
```

That is not a coherent first build.

The revised scope is:

```text
Build a local-first AI workspace on top of a Lukan fork.
Use ZeroClaw as an external sidecar runtime.
Ship one complete channel-to-agent workflow before sync, email, or cloud.
```

## Product Positioning

Lazyboy is a local-first AI workspace where a project channel contains the full
work loop:

```text
message -> task -> approval -> agent run -> artifact -> decision
```

It should feel less like "chat with an agent" and more like a small project
room where humans can see:

- what was requested
- what was approved
- what the agent did
- what files or outputs were produced
- what decision was made afterward

The first version is not a team chat replacement. It is a single-user, local
workspace with team-shaped data structures so the product can grow without
being rewritten.

## Core Bet

The bet is that agent work needs a product container.

Plain agent sessions are too transient:

- the request is detached from the project context
- approvals are hard to audit
- artifacts are scattered
- decisions are not captured
- follow-up work is not naturally linked to the prior run

Lazyboy should make the channel the durable context boundary.

Each channel owns:

- messages
- tasks
- approval requests
- agent runs
- agent events
- artifacts
- decisions

The first useful workflow is:

```text
User posts a project request in a channel.
Lazyboy turns it into a task.
User approves an agent run.
ZeroClaw executes the run.
Lazyboy streams and stores the run events.
Lazyboy records the artifact and final decision in the channel.
```

## What The Code Shows

### Lukan

Lukan is the right base for the shell, but it is not already the Lazyboy
product.

Observed workspace shape:

- `lukan-core`: config, models, crypto, workers, approvals, pipelines
- `lukan-agent`: agent loop, sessions, pipelines, workers, subagents
- `lukan-tools`: shell, file, browser, and task tools
- `lukan-web`: Axum server, REST APIs, WebSocket chat, terminal manager
- `lukan-desktop`: Tauri 2 desktop app
- `lukan-tui`: terminal interface
- `lukan-plugins`: external process plugins
- `lukan-browser`: Chrome DevTools Protocol automation
- `lukan-relay`: relay server
- `desktop-client/`: React/Vite frontend

Useful Lukan assets:

- desktop shell already exists
- web UI already exists
- Tauri integration already exists
- chat, terminal, provider, memory, pipeline, plugin, and approval UI pieces
  already exist
- agent loop and local tools already exist
- background workers and pipeline concepts already exist

Important gaps:

- no durable team workspace product model
- no SQLite product database
- no channel/task/artifact/decision model suitable for Lazyboy
- current tasks are stored as `.lukan/tasks.md`, which is useful for agent-local
  task tracking but not for a product task system
- sessions, approvals, workers, and events are mostly JSON or JSONL files under
  config/data dirs
- Windows is not a first-class platform in the current codebase

Windows-specific risk in Lukan is real. The code and docs contain Unix-shaped
assumptions including:

- `tmux`
- `/proc`
- `/dev/null`
- Unix process groups
- `libc::kill`
- `pre_exec`
- `/bin/bash`
- `mkfifo`
- `$SHELL`
- `pgrep` fallback paths

Conclusion:

```text
Fork Lukan for the app shell and reuse its UI/runtime pieces.
Do not try to force Lazyboy's product model into Lukan's current JSON/session
storage.
```

### ZeroClaw

ZeroClaw is the right agent runtime dependency, but it should not own the
Lazyboy product domain.

Observed workspace shape:

- `zeroclaw-api`
- `zeroclaw-config`
- `zeroclaw-runtime`
- `zeroclaw-gateway`
- `zeroclaw-channels`
- `zeroclaw-tools`
- `zeroclaw-memory`
- `zeroclaw-log`
- `apps/zerocode`
- `apps/tauri`

Useful ZeroClaw assets:

- ACP server over stdio via `zeroclaw acp`
- gateway with ACP over WebSocket at `/acp`
- gateway chat WebSocket at `/ws/chat`
- REST endpoints for events, sessions, logs, config, and tools
- supervised autonomy and permission requests
- session persistence
- Windows setup docs and scripts
- app/service management concepts

Integration conclusion:

```text
Use ZeroClaw as a sidecar.
Talk to it through ACP first.
Use gateway WebSockets second.
Avoid direct internal crate coupling in the MVP.
```

ZeroClaw should own:

- agent session execution
- tool execution
- model/provider interaction
- runtime permission prompts
- low-level run logs

Lazyboy should own:

- channels
- product tasks
- user approvals
- product decisions
- artifacts
- workspace history
- mapping from product tasks to runtime sessions

## First-Version Scope

The first version is a local desktop app for one user and one local workspace.

It must support:

- creating a workspace
- creating channels in that workspace
- posting messages to a channel
- turning a message into a task
- approving or denying an agent run for the task
- starting a ZeroClaw sidecar run
- streaming run events into the channel timeline
- recording generated artifacts
- recording a final decision or follow-up task
- persisting everything locally

It does not need:

- multi-user collaboration
- cloud accounts
- Zenoh replication
- email ingestion
- Slack import
- full workflow YAML
- plugin marketplace
- production auto-update
- team permissions
- mobile app

## MVP Workflow

The first workflow should be coding-oriented because both upstream repos are
already agent/dev-tool shaped.

Example:

```text
Channel: "landing-page-redesign"

User message:
  "Review the current landing page and propose three implementation options."

Lazyboy:
  Creates a task from the message.

User:
  Approves an agent run.

Lazyboy:
  Starts ZeroClaw through ACP.
  Streams run events into the channel.
  Stores artifacts.

Agent output:
  option A: minimal visual refresh
  option B: layout restructure
  option C: full content rewrite

User:
  Chooses option B.

Lazyboy:
  Records the decision and creates a follow-up implementation task.
```

Acceptance criteria:

- the full workflow can be completed without editing files manually
- task status survives app restart
- agent run history survives app restart
- approval and denial are both persisted
- run cancellation is visible in the timeline
- failed sidecar startup has a clear UI state
- generated artifacts are linked from the originating task
- a final decision can be recorded in the channel

## Architecture

Lazyboy should be a Lukan fork with a new product layer, not a thin prompt
wrapper.

Proposed crate/module additions:

```text
lazyboy-core
  Product types and domain rules.

lazyboy-db
  SQLite schema, migrations, repositories, event persistence.

lazyboy-agent-bridge
  ZeroClaw ACP/gateway client, sidecar process management, run event mapping.

lazyboy-platform
  Platform-specific paths, process control, terminal behavior, credentials.

lazyboy-ui
  Product UI surfaces inside the existing React/Tauri app.
```

Existing Lukan pieces to reuse:

- Tauri desktop shell
- React frontend foundation
- provider configuration UI where appropriate
- approval UI patterns
- terminal UI where portable
- local server patterns
- plugin/runtime concepts where useful

Existing Lukan pieces to avoid as product storage:

- `.lukan/tasks.md`
- JSON session files as channel state
- JSON approval files as product audit log
- terminal names/events as product timeline

Those can remain runtime details. Lazyboy needs its own product database.

## Local Database

Use SQLite for the product model.

SQLite is enough for the MVP because:

- the first app is local-first
- the product needs real queries and relationships
- JSON files will become fragile once tasks, approvals, artifacts, and decisions
  are cross-linked
- SQLite can later feed a sync outbox

Minimum tables:

```text
workspaces
  id
  name
  root_path
  created_at
  updated_at

channels
  id
  workspace_id
  name
  purpose
  created_at
  updated_at

messages
  id
  channel_id
  author_type
  body
  created_at

tasks
  id
  channel_id
  source_message_id
  title
  description
  status
  created_at
  updated_at

approvals
  id
  task_id
  agent_run_id
  kind
  prompt
  status
  requested_at
  resolved_at

agent_runs
  id
  task_id
  runtime
  runtime_session_id
  status
  started_at
  finished_at

agent_run_events
  id
  agent_run_id
  sequence
  event_type
  payload_json
  created_at

artifacts
  id
  task_id
  agent_run_id
  kind
  title
  path
  content_ref
  created_at

decisions
  id
  channel_id
  task_id
  title
  body
  created_at

outbox_events
  id
  aggregate_type
  aggregate_id
  event_type
  payload_json
  created_at
  synced_at
```

The `outbox_events` table is intentionally included in the MVP even before
Zenoh. It forces local events to be shaped in a way that can later replicate.

## ZeroClaw Bridge

Primary integration:

```text
spawn `zeroclaw acp`
communicate over JSON-RPC stdio
map ACP session events into Lazyboy agent_run_events
```

Secondary integration:

```text
connect to ZeroClaw gateway `/acp`
use only when stdio ACP is insufficient
```

Avoid for MVP:

```text
embedding ZeroClaw internal crates directly
depending on ZeroClaw database internals
using `/ws/chat` as the main product contract
letting ZeroClaw channels become Lazyboy channels
```

Lazyboy request shape:

```rust
struct AgentRunRequest {
    workspace_id: WorkspaceId,
    channel_id: ChannelId,
    task_id: TaskId,
    prompt: String,
    working_directory: PathBuf,
    risk_profile: RiskProfile,
    approval_policy: ApprovalPolicy,
}
```

Lazyboy event shape:

```rust
enum AgentRunEvent {
    Started { runtime_session_id: String },
    AssistantMessage { content: String },
    ToolRequested { name: String, input_json: String },
    ApprovalRequested { approval_id: ApprovalId, prompt: String },
    ToolCompleted { name: String, output_ref: Option<String> },
    ArtifactCreated { artifact_id: ArtifactId },
    Failed { message: String },
    Cancelled,
    Completed,
}
```

The bridge should be replaceable. Lazyboy should be able to support other
runtimes later without changing the product database.

## Windows Baseline

Windows support must be tested early because Lukan currently has Unix-heavy
process and terminal assumptions.

The first Windows goal is not feature parity. It is a working baseline:

- app starts
- workspace can be created
- SQLite database is stored under an app-owned Windows path
- ZeroClaw sidecar can be discovered or configured
- `zeroclaw.exe acp` can be spawned
- a run can be cancelled
- process cleanup works
- basic terminal works without `tmux`
- unsupported Unix-only features are hidden or clearly disabled

Required platform changes:

- introduce a platform process manager abstraction
- gate `tmux` paths off Windows
- use `portable-pty` or equivalent for shell sessions
- use `ComSpec` or PowerShell as the default Windows shell
- use Windows process/job handling instead of Unix process groups
- replace `/dev/null` assumptions
- avoid `/proc` for process status
- move app data into platform-correct directories

Do not promise persistent tmux-style terminal sessions on Windows in the MVP.

## Explicit Non-Goals

Do not build these in V2 MVP:

- cloud-hosted team service
- multi-tenant accounts
- Slack-compatible chat
- full email client
- send/reply email automation
- Zenoh peer sync
- Postgres backend
- CRDT conflict resolution
- workflow YAML engine
- marketplace
- mobile app
- full Windows feature parity
- deep ZeroClaw crate embedding
- replacing all Lukan UI at once

These are not rejected forever. They are out of scope for the first product
slice.

## Implementation Phases

### Phase 0: Repo Viability Spike

Goal: prove the fork can be built and shaped.

Tasks:

- fork or vendor Lukan as the app base
- build desktop/web on Linux
- run `cargo metadata` and identify workspace boundaries
- add empty Lazyboy crates/modules without changing behavior
- verify where frontend routes and state should live
- add feature gates for clearly Unix-only modules
- document Windows blockers found during compile/check

Exit criteria:

- app still launches locally
- no product feature yet
- no large refactor
- Windows blockers are listed with file paths

### Phase 1: Product Database And Channel Shell

Goal: create the Lazyboy product model.

Tasks:

- add SQLite migrations
- create workspace/channel/message/task tables
- add repository layer
- add local API endpoints
- add channel list UI
- add timeline UI
- add task side panel
- support message-to-task creation
- persist/reload state on restart

Exit criteria:

- user can create a workspace
- user can create a channel
- user can post a message
- user can turn a message into a task
- task state survives restart

### Phase 2: ZeroClaw Sidecar Bridge

Goal: connect one task to one ZeroClaw run.

Tasks:

- add sidecar configuration
- locate configured ZeroClaw binary
- spawn `zeroclaw acp`
- create one ACP session from a task
- stream events into `agent_run_events`
- add approval mapping
- support cancellation
- record failure states

Exit criteria:

- approved task starts a ZeroClaw run
- run events appear in the channel timeline
- approval requests appear in Lazyboy
- denial is persisted
- cancellation works
- app restart shows prior run history

### Phase 3: Windows Baseline

Goal: make the product slice usable on Windows.

Tasks:

- compile the fork on Windows
- hide or gate Unix-only tools
- implement Windows sidecar process management
- verify SQLite path handling
- verify ZeroClaw ACP stdio on Windows
- verify cancellation and cleanup
- verify basic terminal fallback

Exit criteria:

- desktop app starts on Windows
- user can run the Phase 2 workflow on Windows
- unsupported features are explicit, not broken buttons

### Phase 4: Workflow Polish

Goal: make the first workflow feel coherent.

Tasks:

- add artifact panel
- add decision entries
- add retry states
- add run summary
- add task status transitions
- add timeline filters
- add basic redaction for sensitive outputs

Exit criteria:

- user can inspect a completed run without reading logs
- artifact and decision are linked to the source task
- failed runs can be retried or closed

### Phase 5: Sync And Email Later

Goal: expand after the local event model is correct.

Zenoh should start as replication of `outbox_events`, not as a hidden state
mutation layer.

Email should start as read-only ingestion:

```text
email thread imported
  -> channel message
  -> generated summary
  -> suggested task
  -> manual approval to create task
```

Only add send/reply automation after identity, audit, and approval rules are
clear.

## Security Requirements

Lazyboy will run tools that can read files and execute commands. Treat the
local app API as sensitive.

MVP requirements:

- bind local APIs to loopback by default
- keep browser-callable APIs protected
- do not weaken existing CORS restrictions
- persist every approval request and response
- store run events with size limits
- redact secrets from timeline views where possible
- do not pass broad environment variables into sidecars by default
- keep artifacts in app-owned directories
- make workspace file boundaries explicit
- default to supervised ZeroClaw risk profiles

## Main Risks

### Scope Risk

The original idea tries to build too many products at once.

Mitigation:

```text
Ship one local workflow first.
```

### Product Boundary Risk

Lukan sessions and ZeroClaw sessions are runtime concepts. They are not the
Lazyboy product model.

Mitigation:

```text
Lazyboy owns channels, tasks, approvals, artifacts, and decisions in SQLite.
Runtime events are imported into that model.
```

### Windows Risk

Lukan has real Unix assumptions.

Mitigation:

```text
Make Windows a phase before sync/email.
Gate unsupported tools.
Implement platform process handling intentionally.
```

### ZeroClaw Churn Risk

ZeroClaw is moving quickly.

Mitigation:

```text
Use ACP/gateway contracts.
Avoid internal crate coupling until the API stabilizes.
```

### Sync Risk

Distributed sync will multiply mistakes in the local model.

Mitigation:

```text
Build local event persistence first.
Add Zenoh only after replay and conflict rules are clear.
```

## First Build Target

The first build target is:

```text
Lukan fork
+ Lazyboy SQLite product database
+ channel timeline
+ task panel
+ ZeroClaw ACP sidecar run
+ approvals
+ persisted artifacts
```

The first demo should show:

```text
1. Create a workspace.
2. Create a channel.
3. Post a project request.
4. Convert it to a task.
5. Approve a ZeroClaw run.
6. Watch streamed events in the channel.
7. Review generated options/artifacts.
8. Record a decision.
9. Restart the app and see the same state.
```

That is the V2 scope.
