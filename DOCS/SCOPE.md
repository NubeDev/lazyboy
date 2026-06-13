# Lazyboy — Full Scope

Canonical scope. Consolidates the decisions in [IDEA-V3](./IDEA-V3.md) (runtime
choice) and the product vision that followed, into one buildable spec. Where
this contradicts an earlier IDEA doc, **this wins** — the IDEA-V*/SPIKE files
remain as the decision trail, not as live spec.

Status of the runtime question: **Goose is chosen and locked** (see
[Runtime decision](#runtime-decision)). The spike's empirical 9-step run was
not required to land this — the source-level pre-spike findings in
[IDEA-V3-SPIKE](./IDEA-V3-SPIKE.md) plus the durability reframe below settle it.

## One-line product

A local-first team workspace where **every space is a living worker**. You drop
an idea into a space and the whole team — humans and the agent — work it in that
one place: talk becomes tasks, tasks become agent runs, runs ask the team for
approval inline, and results land back as artifacts and decisions. The
conversation does the work instead of being where you coordinate work that
happens elsewhere. Slack's shape, but the messages ship.

## What this is, in one picture

```text
   Gmail · Slack · GitHub · Calendar              the world flows IN
            |  ingress: external events -> messages
            v
   +-------------------------------------------+
   |  SPACE  #new-pricing-page                 |   the unit: one idea, one timeline
   |  -----------------------------------------|
   |  talk -> tasks -> approvals -> agent runs |   everything converges here
   |       -> artifacts -> decisions           |
   |       -> reminders -> calendar            |
   +-------------------------------------------+
            ^  bridge drives goosed over its API
            |
        Goose (rented, unforked, pinned)            the agent: loop, tools, sandbox

   Zenoh fabric: peer-to-peer between teammates, or a central broker for a hub.
   One model, your choice. SQLite is the local source of truth; Zenoh replicates.
```

## The bet

Own the small thing (the team timeline and its trust layer). Rent the big thing
(the agent). Never fork the big thing. Three products are smeared across Slack +
a tracker + email + the IDE + a doc nobody can find; Lazyboy collapses an
initiative into one timeline that both people and an agent act on, with every
agent action gated by a human approval that lives in that same timeline. Nobody
has shipped this well — autonomous agent stacks skip the trust layer, and chat
apps skip the doing. That gap is the product.

## Runtime decision

**Goose (`block/goose`), Apache-2.0, run headless as a pinned sidecar
(`goosed`), unforked.** Drive it over its REST+SSE API today; follow its
ACP-over-HTTP consolidation as it lands.

### Why Goose, honestly

Goose wins on the axes that decide a *foundation*: Block-backed and moving under
the Linux Foundation (longevity, low bus-factor), 70+ MCP tool extensions and a
documented sandbox (we write zero tools), an active ecosystem, and a clean
headless API an external process can drive.

The one place Goose is weak for us is durable pause across a crash. The pre-spike
review confirmed in source that Goose holds a pending tool-approval as an
in-memory `tokio::oneshot`, not persisted state — kill `goosed` mid-approval and
the in-flight continuation is gone. That sounds fatal for an approval-centric
product, and it was nearly the reason to pick the younger Awaken instead. It is
not fatal, for one reason:

**The durable approval row is Lazyboy's, not the runtime's.** The approval lives
as a row in *our* SQLite timeline, written the moment Goose emits the tool
request over SSE. So "the approval survives a crash and stays in the timeline"
is true regardless of Goose. The only thing Goose must do after a crash is
*resume the run* to execute the approved tool — and that is a bounded piece of
bridge code, not an architectural wall (see
[Approvals and the crash-resume seam](#approvals-and-the-crash-resume-seam)).

Awaken would give that resume for free, but it is four months old, single-org,
bring-your-own-MCP, no sandbox. Trading a far stronger, better-supported,
better-tooled base for one bounded piece of bridge code is the wrong trade for a
product foundation. Goose wins.

### The one load-bearing rule: no fork

Talk to Goose only through its API/ACP. Do not fork it, do not embed its
internal crates, do not use its desktop UI. Forking converts "rent the big
thing" into "own the big thing" and inherits a large, fast-moving codebase
forever — the exact mistake [IDEA-V2](./IDEA-V2.md) made with Lukan.

If Goose's durable-approval gap ever needs a code change rather than a bridge
workaround, escalate in this order, loosest first — never jump to a fork:

1. **Own resume in the bridge (default, no Goose change).** Capture the tool
   request from SSE into the `approvals` row; on a mid-approval crash,
   `session/load` and re-drive from Goose's persisted history.
2. **Pin + minimal patch set.** Use the workspace's existing vendored-and-patched
   discipline (`ai-runner.PATCHES.md` pattern: pinned version, every patch
   logged, a marker in source). A small, out-of-loop patch only.
3. **Upstream it.** PR the change to Goose; best outcome if accepted, timeline
   not ours.

A true fork is reserved for the case where many deep changes are needed, at
which point the decision is really "build our own runtime" and reopens
[IDEA-V3](./IDEA-V3.md), not a quiet drift into maintenance.

## Architecture

```text
Lazyboy app (Tauri 2 + React)               ours
  space timeline, task panel, approval queue, artifacts, calendar, reminders
        |
Lazyboy core (Rust, small)                  ours
  spaces, messages, tasks, approvals, agent_runs, artifacts, decisions,
  reminders, calendar, integrations
  -> ONE SQLite db per node (local source of truth)
  -> bridge: drives goosed over its API; imports run events into the timeline
  -> ingress: external events (gmail/slack/github/calendar) -> messages
  -> outbox: every state change -> outbox_events (the sync boundary)
        |
goosed (Goose headless, pinned, unforked)   rented
  agent loop, goose-mcp tools, 70+ MCP extensions, sessions, sandbox

Zenoh fabric (later phase)                  the team layer
  replicates the timeline over outbox_events; peer-to-peer OR central broker
```

### Leverage vs build

| Concern | Provider |
| --- | --- |
| Agent loop, model calls, sessions, sandbox | Goose |
| Tools (file/shell/http/browser/git + MCP ecosystem) | Goose |
| Spaces, messages, timeline | Lazyboy |
| Tasks, approvals, decisions, artifacts, reminders, calendar | Lazyboy |
| Integrations ingress + outbound | Lazyboy (Goose MCP exts for outbound tool actions) |
| Team sync / replication | Zenoh (later) |
| UI | Lazyboy |

We write zero agent code and zero tools. We write the product layer: a handful
of SQLite tables, a timeline view, an ingress layer, and the bridge.

## The space-as-worker model

A **space** is one idea or initiative — `#new-pricing-page`, `#q3-migration`,
`#acme-onboarding`. It is the unit of everything:

- Every message, task, agent run, approval, artifact, decision, reminder, and
  calendar item belongs to exactly one space.
- External events route *into* the relevant space as messages (a PR comment, a
  customer email, an invite). The space is the single source of truth for the
  initiative — "where was that decided?" is answered by scrolling one timeline.
- An agent run is scoped to a space, sees that space's context, and drops its
  output back into the same timeline. Talk in the channel becomes work in the
  channel.

Spaces nest under a **workspace** (a team/org boundary). MVP is one workspace,
one trust boundary, many spaces, many concurrent runs.

The product vocabulary — **feed**, **inbox**, **approval queue**, **user**,
**group**, **space membership**, **feed visibility**, **workshop**,
**workflow**, **automation**, **workflow agent**, **approval policy** — is
defined in [`TERMINOLOGY.md`](./TERMINOLOGY.md), which maps each term to the
concept it names here and flags what is new. The sections below specify the
ones that change the MVP shape.

## Feeds, membership, and visibility (post-step-3)

These extend the space model past the single-tenant MVP; they land after
ingress (build step 3) and are noted in [non-goals](#non-goals-mvp) as
explicitly-deferred, not abandoned.

- **Feed** — an ingress source bound to a space (Slack, email, GitHub,
  calendar). "Feed" is the user-facing name for what
  [Integrations](#integrations) calls ingress; a space can carry several.
- **User / group** — a `human` identity, and a named set of them. Either can
  be granted **space membership**. This is the structure inside the workspace
  trust boundary; it is the first thing past MVP single-tenancy.
- **Feed visibility** — per (feed, space), which users/groups see that feed.
  A private email feed lands only for its owner, who can **share an item**
  (surface one message) or **auto-share a feed** (default it visible to chosen
  users/groups). This is per-user access control *inside* a space, the most
  significant departure from "everyone sees everything," and stays out of MVP
  code (R4) until promoted.

## Workflows and automation (build step 6)

A **workflow** is a saved, triggerable [agent run](#sqlite-domain-model): a
**trigger** (a feed event or a schedule) starts a run with tools, results land
back in the space, and multi-step workflows can place an approval checkpoint
between steps. An **automation** is a workflow that is enabled and live (its
trigger armed). A **workflow agent** is an orchestration run type that watches
feeds, picks which workflow to fire, and sequences the steps — it drives Goose,
it does not replace it; every actual step is still a Goose tool call (R3).

This adds no new trust primitive. Each workflow carries an **approval policy**,
the user's choice per workflow:

- **require approval** (default) — every outside-world step parks as a pending
  `approvals` row, exactly like an interactive run.
- **auto-approve** — the user opts a specific workflow into acting unattended;
  its steps auto-resolve their own `approvals` rows instead of parking. The row
  is still written for audit, so "what did the agent do and on whose authority"
  stays answerable in the timeline.

Auto-approve is a deliberate, scoped exception to R6, chosen per workflow by a
human — not a global "turn the gate off" switch. See the
[non-goals amendment](#non-goals-mvp).

## Approvals and the crash-resume seam

Lazyboy's trust thesis: **an approval is a durable row in the timeline that the
agent waits on, not a blocking RPC.** This is what lets a team point the agent at
real repos and inboxes.

Flow:

1. Goose emits a tool request over the `/reply` SSE stream
   (`MessageContent::ActionRequired`, naming tool + input).
2. The bridge writes an `approvals` row (`pending`) and a `message` of type
   `tool_request` into the space timeline. Nothing is answered yet.
3. The team sees the request inline. Anyone in the workspace can approve or deny
   (single-tenant trust boundary, MVP).
4. On approve, the bridge POSTs the decision to goosed
   (`POST /action-required/tool-confirmation`); Goose executes the tool; the
   bridge imports the result as a `tool_result` message and any output as an
   `artifact`.

Crash-resume (the seam Goose does not give for free): if `goosed` dies between
steps 2 and 4, the `approvals` row and the captured tool request survive in our
SQLite. On restart the bridge reconciles: for any `pending`/`approved` approval
whose run is no longer live, `session/load` the Goose session and re-drive from
its persisted history to the approval point, then apply the decision. This re-drive
path is the bounded bridge work the Goose choice signs us up for; it is built in
the bridge phase, not deferred.

## SQLite domain model

One SQLite db per node, the local source of truth. Runtime state arrives as
imported timeline events; it is never the truth. Minimum tables:

```text
workspaces(id, name, created_at)

spaces(id, workspace_id, slug, title, status, created_at)
  -- one idea/initiative; status: active | archived

identities(id, workspace_id, kind, display_name, external_ref)
  -- who acts: human member or an integration/agent principal.
  -- needed even single-tenant so P2P timelines attribute authorship.

messages(id, space_id, author_identity_id, kind, body, ts, in_reply_to,
         ref_id)
  -- kind: human | agent | system | tool_request | tool_result |
  --       artifact_ref | decision_ref | ingress
  -- ref_id points at the approvals/artifacts/decisions/ingress row when typed

tasks(id, space_id, title, state, created_from_message_id, agent_run_id,
      created_at, updated_at)
  -- state: open | running | blocked_on_approval | done | cancelled

agent_runs(id, space_id, task_id, goose_session_id, status,
           started_at, ended_at)
  -- status: queued | running | waiting_approval | succeeded | failed |
  --         cancelled

agent_run_events(id, agent_run_id, seq, kind, payload_json, ts)
  -- imported from goosed SSE: tool calls, outputs, tokens, notifications

approvals(id, space_id, agent_run_id, goose_session_id,
          tool_name, tool_input_json, status, requested_at, resolved_at,
          resolved_by_identity_id)
  -- status: pending | approved | denied
  -- the trust layer; column set forward-compatible with the spike's table

artifacts(id, space_id, agent_run_id, kind, uri, meta_json, created_at)
  -- kind: file | pr | url | patch | report ...

decisions(id, space_id, message_id, summary, decided_by_identity_id,
          decided_at)

reminders(id, space_id, task_id, due_at, body, status)
  -- status: pending | fired | dismissed

calendar_events(id, space_id, source, external_ref, title, starts_at,
                ends_at, meta_json)
  -- source: local | gcal | ...

integrations(id, workspace_id, provider, account_ref, secret_ref, status,
             config_json)
  -- provider: gmail | slack | github | gcal ...
  -- secret_ref points at the host secrets store, never inline creds

ingress_events(id, integration_id, space_id, external_id, kind, payload_json,
               message_id, received_at)
  -- raw external event, deduped by (integration_id, external_id), mapped to a
  -- message; the idempotency + audit boundary for ingress

outbox_events(id, aggregate, aggregate_id, event_json, seq, created_at,
              synced_at)
  -- every state change appended here; the Zenoh sync boundary (later phase)
```

Keep migrations real but minimal. The UI subscribes to events; it does not keep
its own chat/session store backed by anything but presentation state.

## Integrations

Integrations are **two-way**:

- **Ingress** — external events become messages in a space. A GitHub PR comment,
  a Gmail thread, a Slack message, a calendar invite land in the relevant space.
  Routing rule for MVP: explicit binding (a space subscribes to a repo / label /
  thread / channel); smarter auto-routing is post-MVP. Every ingress event is
  deduped through `ingress_events` so a redelivery never doubles a message.
- **Outbound** — the agent acts back out through Goose MCP extensions (open a PR,
  reply to a thread, create a calendar event). Outbound actions that mutate the
  outside world are gated by the same approval flow as any other tool.

MVP providers, in priority order: **GitHub** and **Gmail** first (highest
signal), then Slack, then Google Calendar. Secrets live in the host secrets
store referenced by `integrations.secret_ref`; never inline.

## Zenoh sync fabric (later phase)

The team layer rides on `outbox_events`, not on bespoke replication.

- **Local-first.** Each node owns its SQLite. Zenoh replicates the timeline by
  shipping `outbox_events`; SQLite stays the source of truth.
- **Peer-to-peer or central broker, one model.** Zenoh peer mode connects
  teammates directly for a small flat team; router/broker mode gives a hub for a
  larger org. Same event model either way — the topology is configuration.
- **Ordering and conflict.** `outbox_events.seq` per aggregate gives per-space
  ordering; messages are append-only so merges are union. Mutable rows (task
  state, approval status) need a defined last-writer/merge rule — an open
  question to settle before Zenoh turns on, not during.

Hard rule from [IDEA-V3](./IDEA-V3.md): **no Zenoh until the local event model is
stable.** The single-node timeline + outbox must be solid first, or sync will
amplify every local modelling mistake.

## UI: one React app, two shells

The product face is a Claude-cowork-shaped workspace: a left rail of spaces,
a central space timeline where talk, agent messages, tool requests, and
results interleave, inline **approval cards** that gate every outside-world
action, and a right panel for the space's tasks and live runs. One React 19 +
TypeScript app (`ui/lazyboy-ui/`, Vite, Tailwind, shadcn/ui) ships to **both**
a Tauri 2 desktop shell and the browser — there are no per-shell UI files
(inherits the codeless R3 discipline).

The app imports exactly one boundary type, `RpcClient`. The shell injects the
implementation:

```text
ui/lazyboy-ui/  (React 19 + TS + Tailwind + shadcn)   one codebase
  imports ONLY RpcClient — never tauri APIs, never fetch() to core directly
        |
        +-- desktop: TauriRpcClient   -> Tauri 2 commands -> lazyboy-core in-process
        +-- browser: HttpRpcClient    -> axum HTTP+SSE server (CORS) -> lazyboy-core
        +-- dev:     MockRpcClient     -> in-memory cowork fixture, no backend
```

`RpcClient` surface (mirrors the core engine + store reads):

```text
listSpaces() -> Space[]
timeline(spaceId) -> Message[]            // append-only, ordered
listPending(spaceId) -> Approval[]
listTasks(spaceId) -> Task[]
listRuns(spaceId) -> AgentRun[]
startRun(spaceId, prompt) -> RunOutcome
decide(approvalId, status) -> RunOutcome
subscribe(spaceId, cb) -> unsubscribe     // SSE on browser, Tauri event on desktop
```

The wire enums are the snake_case serde forms already on the Rust types
(`MessageKind`, `RunStatus`, `TaskState`, `ApprovalStatus`) — the TS string
unions are kept identical so the same JSON crosses both transports unchanged.

CORS is a first-class concern, not an afterthought: the browser shell is served
from a different origin than the axum core, so the HTTP server sets permissive
CORS for the single-tenant bearer (SCOPE R4) and the SSE endpoint streams the
same `subscribe` events the Tauri shell delivers over its event channel. Build
the `MockRpcClient` first so the full shell renders with no backend, then land
the two real transports behind the same interface.

_Status: the React app (`ui/lazyboy-ui/`) is built and green — the full
cowork shell (space rail, timeline with inline approval cards, task/run
panel), the `RpcClient` boundary, the `MockRpcClient` fixture, and both
real-transport clients (`shell/http.ts`, `shell/tauri.ts`) behind a
shell-side `selectClient()`. It runs in the browser today against the mock.
The open item is the two backend shells the real clients target: an
`lazyboy-server` axum crate (HTTP + SSE + CORS over the existing core) and a
Tauri 2 desktop crate exposing the same surface as commands + an event
channel._

## Cross-cutting rules

- **R1 — SQLite is the source of truth.** Job/task/approval/event state lives in
  SQLite. The UI subscribes; it does not hold authoritative state.
- **R2 — Single transport, single client interface (UI).** The React UI talks to
  core through one client interface (`RpcClient`); it never reaches into Goose,
  never imports a shell-specific transport (`@tauri-apps/api`, raw `fetch` to
  core) directly. The shell injects the implementation. (Inherits the codeless
  R2 boundary; see [UI: one React app, two shells](#ui-one-react-app-two-shells).)
- **R3 — Never fork Goose.** Only the API/ACP seam. See
  [the no-fork rule](#the-one-load-bearing-rule-no-fork).
- **R4 — Single-tenant trust boundary (MVP).** One workspace, one bearer of
  trust, many concurrent runs. No per-job auth scopes or multi-tenant isolation
  in MVP code.
- **R5 — Secrets never inline.** Integration creds live in the host secrets
  store, referenced by id.
- **R6 — Approval gates every outside-world mutation.** Any tool call that
  changes a repo, an inbox, a calendar, or the filesystem outside the sandbox
  goes through the `approvals` flow. The one sanctioned exception is a workflow
  a human has set to **auto-approve** (see
  [Workflows and automation](#workflows-and-automation-build-step-6)): the
  `approvals` row is still written for audit, but it auto-resolves instead of
  waiting. The gate is the default and the only path for interactive runs;
  auto-approve is an explicit per-workflow opt-in, never global.

## Build order

Each step is independently demoable. The trap is building all of it at once and
shipping none; this order ships value at step 1.

1. **One space, local, with Goose.** SQLite + space timeline + task panel +
   the approval-in-thread loop, driving `goosed` for a single space. Survives
   restart, including the crash-resume reconcile for a pending approval. This is
   the magic moment — prove it on one machine.
   _Status: the engine, store, host transport, and a thin `lazyboy` CLI
   shell (`crates/lazyboy-cli`: init / run / approve / deny / pending /
   timeline) are built and green. The whole stack is verified end to end
   against an in-process fake goose (`GOOSE-ACP.md` "Test strategy"). The
   one open item is running it against a live `goose serve` with a
   configured provider — blocked in CI by the sandbox refusing to launch
   the binary, not by missing code._
2. **Bridge hardening.** Full run-event import (SSE -> `agent_run_events`),
   artifacts imported, cancel and retry, the approval queue view.
3. **Integrations as ingress.** GitHub + Gmail first, as messages flowing into
   bound spaces, deduped through `ingress_events`. Then outbound via MCP exts,
   gated by approvals.
4. **Calendar, reminders, decisions** as the space's durable memory.
5. **Zenoh sync.** Peer-to-peer and broker modes, once the local event model is
   stable and the mutable-row merge rule is decided.
6. **Workflows, automation, and membership.** Triggers (feed + schedule), saved
   workflows with a per-workflow approval policy, the workflow agent, and the
   user/group + feed-visibility model. Built once feeds (step 3) exist to
   trigger on and the team layer (step 5) gives more than one human to share a
   feed with.

Out until step 1 works end to end: Zenoh, multi-workspace, mobile shells,
auto-routing of ingress, anything multi-tenant.

## Non-goals (MVP)

- Multi-tenant isolation and OIDC — single bearer trust boundary for MVP.
  (Per-user structure — users, groups, space membership, feed visibility —
  is *deferred, not abandoned*: it lands at build step 6, after the team layer
  exists, and stays out of MVP code under R4 until then.)
- Globally autonomous (no-human) agent operation — the approval gate is the
  product default and the only path for interactive runs. The single sanctioned
  exception is a workflow a human has explicitly set to auto-approve (R6); that
  is a scoped, per-workflow, audited opt-in, not a switch that turns the gate
  off across the product.
- Forking, embedding, or shipping Goose's internals or desktop UI.
- Bespoke replication — Zenoh over `outbox_events`, evaluated only after the
  local model is stable.
- Building any agent tooling Goose already provides.

## Open questions

1. **Mutable-row merge under Zenoh.** Last-writer-wins vs. a small CRDT for task
   state and approval status. Must be answered before sync turns on.
2. **Re-drive determinism.** When the bridge re-drives a Goose session after a
   mid-approval crash, does the same tool call reliably reappear, or is a
   capture-and-replay-the-tool-result approach needed for the edge case? Confirm
   empirically during the bridge phase (this is the deferred part of the spike).
3. **Ingress auto-routing.** MVP uses explicit space bindings; when does
   content-based routing earn its complexity?
4. **Identity under P2P.** How identities are established and trusted across
   peers without a central broker.

## Pointers

- Runtime decision and rationale: [IDEA-V3](./IDEA-V3.md)
- Runtime bake-off + pre-spike findings: [IDEA-V3-SPIKE](./IDEA-V3-SPIKE.md)
- Earlier product scope and domain model: [IDEA-V2](./IDEA-V2.md)
- Original concept: [IDEA](./IDEA.md)
