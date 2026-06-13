Final call
Lukan = main app / team workspace fork
ZeroClaw = agent runtime sidecar
Zenoh = peer/cloud sync

Lukan is the product base because it is already a Rust AI workstation with terminal, browser, messaging, background workers, sandboxed execution, Tauri-style app direction, and local-first shape.

ZeroClaw is the agent engine because it is a Rust-first autonomous agent runtime with pluggable providers, channels, tools, memory and observers, and recent releases show active work on runtime/session/CLI behaviour.

Architecture
Your Product
├─ Lukan fork
│  ├─ teams
│  ├─ channels
│  ├─ messages
│  ├─ tasks
│  ├─ approvals
│  ├─ emails
│  ├─ files/artifacts
│  ├─ desktop UI
│  └─ local DB
│
├─ ZeroClaw sidecar
│  ├─ Claude Code / Codex / Gemini CLI runs
│  ├─ shell/browser/http tools
│  ├─ MCP tools
│  ├─ long-running agent jobs
│  └─ tool execution events
│
└─ Zenoh
   ├─ local peer sync
   ├─ cloud relay later
   └─ workspace event bus
Ownership rule

This is the key bit:

Thing	Owner
Teams	Lukan / your app
Channels	Lukan / your app
Messages	Lukan / your app
Tasks	Lukan / your app
Approvals	Lukan / your app
Emails	Lukan / your app
Agent execution	ZeroClaw
Tool execution	ZeroClaw
MCP tools	ZeroClaw
Claude Code / Codex / Gemini wrappers	ZeroClaw sidecar
Sync	Zenoh

Do not let ZeroClaw own your task model.
Do not let Lukan duplicate ZeroClaw’s agent runtime.

Day-one flow
Aidan sends message in channel
        ↓
Lukan creates message + detects task intent
        ↓
Lukan creates task and approval state
        ↓
Lukan asks ZeroClaw sidecar to run agent job
        ↓
ZeroClaw runs Claude Code / design agent / MCP tools
        ↓
ZeroClaw streams events back
        ↓
Lukan stores events as agent_run + messages + artifacts
        ↓
Aidan approves / gives feedback
        ↓
Lukan updates task and may trigger another ZeroClaw run
Windows work

Yes, Lukan needs Windows hardening. Treat this as one of the first engineering tracks.

Focus on:

1. Tauri Windows build
2. Windows paths: AppData/Local, AppData/Roaming
3. ConPTY terminal support
4. PowerShell/cmd command execution
5. process kill / job objects
6. Windows service or tray process
7. ZeroClaw sidecar binary management
8. installer + auto-update
9. browser detection: Edge/Chrome
10. secrets storage using Windows Credential Manager or Tauri Stronghold
Repo layout I’d use
lukan-workspace-fork/
  crates/
    lukan-core/
    lukan-desktop/
    lukan-agent/
    lukan-plugins/

    team-core/
      teams
      channels
      messages
      tasks
      approvals
      emails
      artifacts

    team-db/
      sqlite
      migrations

    team-zeroclaw/
      sidecar_manager
      rpc_client
      event_mapper
      job_dispatch

    team-sync-zenoh/
      topic_map
      peer_sync
      conflict_resolution

    team-windows/
      conpty
      service
      credential_store
      process_manager
Minimum integration contract

Make ZeroClaw talk to Lukan through a small stable interface:

struct AgentJobRequest {
    workspace_id: String,
    channel_id: String,
    task_id: Option<String>,
    prompt: String,
    tools_allowed: Vec<String>,
    approval_required: bool,
}

enum AgentJobEvent {
    Started,
    ThoughtSummary(String),
    ToolCall { name: String, args_json: String },
    ToolResult { name: String, output: String },
    ArtifactCreated { path: String, kind: String },
    NeedsApproval { reason: String },
    Failed { error: String },
    Completed { summary: String },
}

That boundary keeps the system clean.

Product direction

So yes:

Fork Lukan.
Run ZeroClaw sidecar from day one.
Use Zenoh for sync from day one.
Port/harden Lukan for Windows from day one.

This gives you an existing Rust project as the base, keeps ZeroClaw updates possible, and still lets your product own the Slack/Asana/email replacement model.





Yep. Here is the clean overview.

# Product idea

You are building a **Rust local-first team workspace** that replaces:

```text
Slack + Asana + email + AI coding/design agents
```

But the key difference is:

> **Chat is not just chat. Every conversation can become tasks, approvals, agent runs, files, emails and decisions.**

So instead of a team working across Slack, Asana, Gmail, Claude Code, Codex, OpenClaw, etc., they work inside one channel.

## Core stack

```text
Lukan fork        = main product shell
ZeroClaw sidecar  = agent/tool runtime
Zenoh             = local-first peer/cloud sync
SQLite            = local state
Postgres          = optional cloud/team state later
Tauri             = Windows/macOS/Linux desktop app
Rust              = main language
```

## Ownership split

| Area                             | Owner            |
| -------------------------------- | ---------------- |
| Teams                            | Lukan / your app |
| Channels                         | Lukan / your app |
| Messages                         | Lukan / your app |
| Tasks                            | Lukan / your app |
| Approvals                        | Lukan / your app |
| Emails                           | Lukan / your app |
| Files/artifacts                  | Lukan / your app |
| Agent execution                  | ZeroClaw         |
| Claude Code / Codex / Gemini CLI | ZeroClaw sidecar |
| Shell/browser/MCP tools          | ZeroClaw sidecar |
| Local/cloud sync                 | Zenoh            |

The rule is simple:

```text
Lukan owns the work.
ZeroClaw does the work.
Zenoh moves the work.
```

---

# Main product model

A **channel** is the main object.

Not a Slack-style room only.

```text
Channel
├─ messages
├─ tasks
├─ approvals
├─ emails
├─ agent runs
├─ files
├─ decisions
├─ notes
└─ audit log
```

Example channel:

```text
# dashboard-redesign
```

Inside that channel you have:

```text
- Aidan’s request
- Lina’s task
- 3 AI design ideas
- Aidan’s selected option
- Claude Code / design-agent run
- screenshots or Figma/design output
- feedback
- related customer emails
- final decision log
```

That is the product.

---

# Main workflow idea

## 1. Message becomes work

You write:

```text
Lina, design me a new HVAC dashboard. Give me 3 ideas first and tag me before starting.
```

System creates:

```text
Message
→ Task
→ Brief
→ Approval needed
→ Agent suggestion job
```

Task:

```yaml
title: Design HVAC dashboard concept
assignee: Lina
status: waiting_for_options
requested_by: Aidan
channel: dashboard-redesign
approval_required: true
```

---

## 2. AI gives options

The system posts:

```text
Here are 3 design directions:

1. Operations overview
2. Energy insights dashboard
3. Fault detection / action centre

Aidan, pick one before Lina or the agent starts.
```

You pick:

```text
Go with option 2, energy insights dashboard.
```

System records:

```text
Decision: Aidan selected option 2
Task status: approved_to_start
```

---

## 3. Human or agent runs the work

Lina can do it manually, or she can attach an agent:

```text
Run design agent using Claude Code.
Use dashboard-redesign brief.
Generate React/Svelte screen.
Post result back here.
```

Lukan sends job to ZeroClaw:

```text
AgentJobRequest
├─ channel_id
├─ task_id
├─ prompt
├─ allowed_tools
├─ approval_required
└─ output_type: design_artifact
```

ZeroClaw runs:

```text
Claude Code / Codex / Gemini CLI
browser tool
file tool
MCP tools
shell tools
```

Then streams events back into the channel.

---

## 4. Result comes back into the channel

The channel gets:

```text
Agent run started
Tool used: Claude Code
Files changed
Screenshot generated
Design artifact uploaded
Summary posted
```

Then you reply:

```text
Looks good, but make the left nav cleaner and add site-level energy KPI cards.
```

System creates:

```text
Subtask: clean up left nav
Subtask: add energy KPI cards
Status: revision_needed
```

Then the loop continues.

---

# Email workflow idea

This is one of the biggest wins.

## Current bad workflow

```text
Customer email in Gmail
→ someone forwards to Slack
→ someone creates Asana task
→ someone updates designer/dev
→ context gets lost
```

## Your workflow

```text
Email comes in
→ system reads/summarises it
→ matches it to a channel
→ posts it into the channel
→ creates task if action is needed
```

Example:

```text
Email from Fujitsu:
"Can you send the updated dashboard mockup before Friday?"
```

System posts in `#dashboard-redesign`:

```text
New email matched to this channel.

From: Fujitsu
Summary: They need the updated dashboard mockup before Friday.
Suggested task: Send updated dashboard mockup.
Suggested assignee: Lina
Due: Friday
```

Then it asks:

```text
Create this task?
```

Or if rules allow auto-create:

```text
Task created: Send updated dashboard mockup to Fujitsu
```

---

# Good workflow types to support

## 1. Design request workflow

```text
Message request
→ create task
→ generate 3 ideas
→ wait for Aidan approval
→ run design agent / assign designer
→ post artifact
→ feedback loop
→ final approval
```

Best for:

```text
UI design
dashboard design
marketing assets
product mockups
website pages
```

---

## 2. Coding task workflow

```text
Message request
→ create coding task
→ generate implementation plan
→ approval gate
→ run Claude Code / Codex / Gemini
→ create branch/patch
→ run tests
→ post summary
→ human review
```

Example:

```text
Build the new task approval state machine.
```

System creates:

```text
Task: Build approval state machine
Agent: Claude Code
Workflow: code-change
Approval: required before file changes
```

---

## 3. PR review workflow

```text
GitHub PR event
→ channel update
→ agent reviews PR
→ posts risks
→ creates review tasks
→ human approves comments
→ comments posted to GitHub
```

This fits your earlier idea of reusable YAML workflows.

Example:

```yaml
workflow: pr-review
inputs:
  repo: nube-io/app
  pr: 421
  review_level: hard
steps:
  - summarise_diff
  - find_risks
  - run_tests
  - propose_comments
  - wait_for_approval
  - post_review
```

---

## 4. Customer/support workflow

```text
Email/support message comes in
→ classify
→ match project/channel
→ summarise
→ create task
→ assign owner
→ draft reply
→ wait for approval
→ send response
```

Example:

```text
Customer says HVAC dashboard values look wrong.
```

System creates:

```text
Task: Investigate wrong HVAC dashboard values
Assignee: support/dev
Agent suggestion: check logs, DB values, recent deployments
```

---

## 5. Meeting-to-tasks workflow

```text
Meeting notes/transcript
→ extract decisions
→ extract tasks
→ assign owners
→ add due dates
→ post to channel
```

Output:

```text
Decisions:
- Use energy insights layout
- Keep left nav minimal

Tasks:
- Lina: update design
- Tan: check backend data shape
- Aidan: approve final mockup
```

---

## 6. Long-running agent workflow

This matches your previous AI job-loop ideas.

```text
User starts workflow
→ agent breaks job into stages
→ each stage creates a tick
→ run one safe chunk
→ commit/save output
→ schedule next tick
→ stop on blocker or approval
```

Example:

```text
Migrate old dashboard to new frontend.
```

Workflow:

```yaml
workflow: dashboard-migration
provider: zeroclaw
agent: claude-code
blocker_policy: stop
stages:
  - inspect_old_dashboard
  - create_spec
  - map_data_sources
  - build_new_ui
  - test
  - request_review
```

This should be first-class in the app, not hidden in logs.

---

## 7. Approval workflow

Approvals should be everywhere.

```text
Agent wants to send email
→ approval required

Agent wants to edit files
→ maybe approval required

Agent wants to run shell command
→ maybe approval required

Agent wants to post customer reply
→ approval required
```

Approval object:

```yaml
approval:
  type: send_email
  requested_by: agent
  approver: Aidan
  status: waiting
  context: customer_reply
```

This gives trust.

---

# Channel types

You could have different channel types:

```text
Project channel
Customer channel
Team channel
Private DM
Agent channel
Inbox channel
Incident channel
Workflow channel
```

## Project channel

```text
# dashboard-redesign
```

Has tasks, files, agent runs, emails, decisions.

## Customer channel

```text
# fujitsu-dashboard
```

All emails, tasks, meetings and agent work for that customer.

## Agent channel

```text
@design-agent
@code-agent
@email-agent
```

You talk directly to an agent, but the output can attach to a project channel.

## Inbox channel

```text
# inbox
```

All new emails/messages that have not been matched yet.

## Incident channel

```text
# incident-site-down
```

Agent can collect logs, create tasks, assign people, summarise timeline.

---

# MVP workflow

Build one great workflow first:

```text
Message → task → 3 options → approval → ZeroClaw run → artifact → feedback
```

That proves the whole system.

MVP screens:

```text
1. Channel view
2. Task panel
3. Agent run panel
4. Approval queue
5. Email feed
6. Artifact/files panel
```

Channel layout:

```text
------------------------------------------------
# dashboard-redesign

[Messages / timeline]

Aidan: Lina, design a new dashboard...
System: Task created
AI: Here are 3 options...
Aidan: Pick option 2
Agent: Running Claude Code...
Agent: Design uploaded
Aidan: Make nav cleaner

------------------------------------------------
Right panel:
Task: Design HVAC dashboard
Status: revision_needed
Assignee: Lina
Approvals: 1 pending
Agent runs: 2
Emails: 3 matched
Files: 5
------------------------------------------------
```

---

# Day-one build plan

## Phase 1 — Lukan Windows port

```text
- get Lukan building on Windows
- fix paths
- ConPTY terminal
- PowerShell/cmd execution
- process manager
- app data directories
- installer
- secrets storage
```

## Phase 2 — Product model

Add:

```text
teams
channels
messages
tasks
approvals
email_threads
agent_runs
artifacts
audit_log
```

## Phase 3 — ZeroClaw sidecar

Add:

```text
sidecar manager
job dispatch
event stream
job cancellation
tool approval mapping
artifact import
```

## Phase 4 — Zenoh sync

Add topics like:

```text
team/{team_id}/channel/{channel_id}/message
team/{team_id}/task/{task_id}/updated
team/{team_id}/agent_run/{run_id}/event
team/{team_id}/email/{thread_id}/ingested
team/{team_id}/artifact/{artifact_id}/created
```

## Phase 5 — Email ingestion

Add:

```text
email account connection
thread matching
summary
task suggestion
auto-channel attach
approval before reply/send
```

---

# Best product positioning

Do not call it only a Slack alternative.

Better:

> **A Rust local-first AI workspace where team chat, tasks, email and agents live in the same channel.**

Or:

> **A team operating system where every message can become a task, every task can run an agent, and every email lands in the right project.**

That is the real idea.
