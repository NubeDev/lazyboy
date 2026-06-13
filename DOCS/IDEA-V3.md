# Lazyboy IDEA-V3

Decision doc. Supersedes the architecture in IDEA-V2 on two points:

- **Lukan is dropped.** No UI fork.
- **The agent runtime is Goose, run headless as a sidecar — not forked.**

Everything in IDEA-V2 about product scope, the SQLite domain model, async
approvals, and the non-goals still holds. This doc only locks the base.

## One-line product

A local-first team workspace where every channel is a job: you talk in the
channel, the AI turns talk into tasks, asks for approval, runs tools to do the
work, and drops results back in the channel as artifacts and decisions. Slack's
shape, but the messages do things.

## The decision

Leverage an existing agent runtime for the hard half (agent loop, tools,
sessions, sandbox). Build only the team/channel product layer, which does not
exist anywhere and is the actual product.

Chosen runtime: **Goose** (block/goose), Apache-2.0.

Run it **headless as a pinned sidecar** (`goosed`), driven over its API and/or
ACP. Do not fork it. Do not embed its internal crates. Do not use its desktop
UI.

### Why Goose over the alternatives

| | Goose | ZeroClaw | Awaken |
| --- | --- | --- | --- |
| Support / maturity | Block-backed, mature | feature-rich, internal crates moving | small, young |
| License | Apache-2.0 | unknown (risk) | MIT/Apache |
| Headless server | `goosed` (REST/WS) | gateway | server mode |
| Embed as library | `goose` core crate | resists forking | library mode |
| ACP | yes | yes | yes |
| Built-in tools | `goose-mcp` + 70+ MCP exts | native tools | bring-your-own MCP |
| Sandbox | documented sandbox | risk profiles | none |

Awaken's only real edge is its HITL "mailbox suspension" approval model, which
maps cleanly to a durable approval row. That single advantage does not outweigh
Goose's support and built-in tooling. Because all three speak ACP, Goose is not
a lock-in: build against `goosed`/ACP, never reach into internals, and swapping
later is a config change.

## Architecture

```text
Lazyboy app (Tauri 2 + React)        -- ours
  channel timeline, task panel, approval queue, artifacts
        |
Lazyboy core (Rust)                  -- ours, small
  channels, messages, tasks, approvals, decisions, artifacts
  -> ONE SQLite db (source of truth)
  -> bridge: drives Goose over goosed API / ACP
        |
goosed (Goose headless, unforked, pinned)
  agent loop, goose-mcp tools, 70+ MCP extensions, sessions, sandbox
```

## Leverage vs build

| Concern | Provider |
| --- | --- |
| Agent loop, model calls | Goose |
| Tools (file/shell/http/browser/git) | Goose (`goose-mcp` + MCP ecosystem) |
| Sandbox, sessions, agent memory | Goose |
| Channels, messages, timeline | Lazyboy |
| Tasks, approvals, decisions, artifacts | Lazyboy |
| UI | Lazyboy |
| Sync (later only) | `cr-sqlite` or Matrix, not now |

We write zero agent code and zero tools. We write the product layer, which is a
handful of SQLite tables and a timeline view.

## Integration options (pick the loosest that works)

| Mode | Surface | Trade-off |
| --- | --- | --- |
| Sidecar (recommended) | `goosed` REST/WS, or ACP | decoupled, swappable, no fork |
| In-process | `goose` core crate | tighter, harder to swap; only if the seam proves too loose |

Default to the sidecar. Move in-process only with a concrete reason.

## The one load-bearing rule

Own the small thing (the team timeline). Rent the big thing (the agent). Never
fork the big thing. Only ever talk to Goose through its API or ACP. This is what
makes "don't start from zero" sustainable instead of inheriting a whole codebase
forever -- the mistake IDEA-V2 made with Lukan.

## SQLite domain model

Unchanged from IDEA-V2. Product owns its own SQLite; runtime state arrives as
imported timeline events. Minimum tables: workspaces, channels, messages, tasks,
approvals, agent_runs, agent_run_events, artifacts, decisions, outbox_events.

`outbox_events` remains the future sync boundary. No Zenoh until the local event
model is stable. When sync is needed, evaluate `cr-sqlite` (CRDT SQLite) or
Matrix as the event model before building a bespoke replication layer.

## Approvals

Lazyboy's thesis is approvals living in the channel timeline. Model an approval
as a durable row the agent waits on, not a blocking RPC, so a crash mid-approval
is recoverable and the timeline is always the truth.

The open question that decides the bridge design: does `goosed`/ACP cleanly
surface "the agent wants to run tool X -- approve?" to an external product UI, or
is its approval UX welded to its own CLI/desktop flow? If approvals round-trip
over the API, Goose is a clear fit. If not, Awaken's mailbox model earns a second
look. This must be confirmed in the spike before any product code.

## Build order

Each step is independently demoable.

1. Spike: start `goosed`, send one prompt, stream events to stdout. Confirm
   headless drive works and that approval requests round-trip over the API.
2. Product core: SQLite + channel timeline + task panel. No agent. Survives
   restart.
3. Bridge: wire `goosed` in. message -> task -> approval -> Goose run ->
   streamed tool events stored in the timeline -> artifacts imported.
4. Polish: approval queue, decisions, cancel, retry. Ship one vertical slice
   end to end (`#dashboard-redesign`).

Out until step 4 works: Zenoh sync, email ingestion, Windows-as-a-phase,
multi-tenant.

## Prior art: Zylos (zylos-ai/zylos-core)

Zylos is the closest shipped thing to Lazyboy, and looking at it sharpens the
decision rather than changing it. It rents Claude Code / Codex underneath
(switchable at runtime) and wraps them in exactly Lazyboy's product layer:
multi-channel comms routed through one bridge with a SQLite audit trail. Same
"own the timeline, rent the agent" bet, already built — in Node, not Rust.

It is **not a runtime candidate** for the sidecar slot, and must not enter the
bake-off in [IDEA-V3-SPIKE](./IDEA-V3-SPIKE.md):

- No human-in-the-loop. It is autonomous by design — no approval primitive,
  which is Lazyboy's entire thesis. It fails the spike's gate before it runs.
- Not a headless, embeddable engine. It is a PM2 + Caddy + tmux daemon stack,
  Linux/macOS only — the same Unix/tmux Windows problem V2 flagged in Lukan.
- Couples over a bespoke C4 / HXA protocol, not ACP/MCP. That violates the
  never-fork / standard-seam rule the moment you depend on it.

Its value is as a contrast (it shows what Lazyboy is choosing *not* to be:
autonomous vs approval-gated) and as two design ideas worth borrowing **later**,
not in the MVP:

- **Context-threshold memory save.** Zylos auto-checkpoints agent memory at
  ~75% context utilization into a tiered store. Relevant when Lazyboy grows
  long-running agent runs; revisit when `agent_runs` need durable mid-run
  memory, not before.
- **Idle-gated scheduler.** Cron plus natural-language scheduling that only
  dispatches while the user is idle. Relevant to the long-running job-loop
  workflow, post-MVP.

Both are README-level observations, not code-inspected. Before borrowing
either, clone and read the source at a pinned commit, matching the rigor V2
used for Lukan/ZeroClaw. The capability vocabulary ("unified consciousness",
"HXA-Connect B2B federation") is marketing until verified in code.

## Recommended next step

Run the spike. Confirm an external app can drive a `goosed` run and approve a
tool call over its API. If that works, this plan is green and step 2 can start.
