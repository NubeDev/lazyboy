# Goose features vs. Lazyboy — what to rent, what to keep, what to skip

Goose ships five user-facing building blocks beyond the bare agent loop:
**recipes, skills, apps, scheduler, extensions**. Several overlap concepts
Lazyboy has already named ([TERMINOLOGY.md](./TERMINOLOGY.md)) or built
(workflows/automation, build step 6 — [WORKFLOWS.md](./WORKFLOWS.md)). This
doc decides, feature by feature, whether to rent it from Goose, keep our
own, or leave it out — applying the one rule that already settled the
runtime: **rent the agent and its capabilities; own only the team-and-trust
layer** (spaces, the durable approval row, the timeline, membership).

Probed against the pinned `bin/goose` (v1.37.0); wire details for the seam
are in [GOOSE-ACP.md](./GOOSE-ACP.md).

## The deciding question for every feature

Goose's recipes and scheduler run a **fresh agent with its own approval
mode** — `auto` by default, no durable gate, no space binding, no import
into our timeline. That is exactly the gap the runtime decision already
called out ([SCOPE.md "Why Goose, honestly"](./SCOPE.md#why-goose-honestly)):
Goose holds approvals in memory and runs unattended; the durable approval
row and the audit trail are **ours**.

So the test for renting any Goose execution feature is:

> Does it run through Lazyboy's gated ACP drive loop (the
> `session/request_permission` seam → durable `approvals` row → resolve),
> or does it run *around* it?

Anything that runs around the seam violates **R6** (the approval gate) and
**R1** (SQLite is the source of truth — a Goose-scheduled run leaves no
timeline row). That single test, not taste, decides each row below.

(Rule numbers throughout are SCOPE.md's canonical set — R1 SQLite, R2
single transport/UI, R3 never fork Goose, R4 single-tenant, R5 secrets, R6
approval gate. SCOPE wins where CLAUDE.md's workspace numbering differs.)

## Verdicts at a glance

| Goose feature | Closest Lazyboy concept | Verdict |
|---|---|---|
| **Extensions** (MCP tool servers) | `tool` — already defined as "a Goose MCP extension" | **Already rented.** Keep. Extend the *Lazyboy* MCP server, not Goose. |
| **Recipes** (portable workflow YAML: prompt + extensions + params + subrecipes) | `workflow` (`steps_json`, params, approval checkpoints) | **Rent the format, own the wrapper.** Make a workflow = a Goose recipe + trigger + approval policy. Stop hand-rolling step shape. |
| **Skills** (reusable instruction sets that shape agent behavior) | *(none)* | **Adopt — genuine gap.** The "how to behave" layer Lazyboy lacks. Rent it. |
| **Scheduler** (cron-runs a recipe via a fresh agent) | `schedule` trigger + the stage-C always-on daemon (not built) | **Do NOT rent. Build the thin tick.** Goose's scheduler runs around our gate. The cron tick is part of the trust layer. |
| **Apps** (MCP Apps: interactive UI inside Goose Desktop) | *(none — and conflicts)* | **Skip.** Desktop-coupled UI breaks R2 (one React UI, single client interface, four shells). |
| **Subagents** (parallel helper agents within a run) | *(none needed)* | **Allow, don't model.** The rented agent uses them internally; gate their outside-world actions like any tool. |

Sources: [Goose docs](https://goose-docs.ai/),
[Scheduler (DeepWiki)](https://deepwiki.com/block/goose/4.1.5-scheduler-and-recurring-tasks),
[Unify-execution discussion #4389](https://github.com/block/goose/discussions/4389).

## Extensions — already rented, keep going

`tool` in [TERMINOLOGY.md](./TERMINOLOGY.md#tool) is already "a Goose MCP
extension; we write zero tools." Nothing to change in principle. The one
thing we *do* host is the **Lazyboy MCP server** (`/mcp`,
[GOOSE-ACP.md](./GOOSE-ACP.md#mcp-servers--the-lazyboy-tool-seam)) — the
agent's only door back into our own domain. That is not a violation of
"write zero tools": every *outside-world* tool stays a Goose extension; the
Lazyboy MCP server is the trust-layer surface, the exception that lets the
agent act on the space it is scoped to.

Action: continue the keystone roadmap (B/C/D — richer overview,
`set_reminder`/`record_decision`/`list_messages`, cross-space chat). No
Goose-side work.

## Recipes — rent the format, own only the wrapper

A Goose **recipe** is a portable YAML config: a prompt/instructions, the
extensions it needs, typed **parameters**, optional **retry/validation**,
and **subrecipes** for composition. A Lazyboy **workflow** is a saved
triggerable agent run with `steps_json`, params, and approval checkpoints —
i.e. a recipe plus a trigger plus an approval policy. The overlap is near
total, and the parts that overlap (prompt, required extensions, params,
sub-steps) are precisely the parts Goose maintains for free.

**Proposal (gated on the probe in open question 2 — not yet a settled
decision): a Lazyboy workflow should *be* a Goose recipe wrapped by our two
trust-layer additions — the trigger and the approval policy.** This holds
only if a recipe driven over the ACP seam still surfaces every gated tool;
that path is unproven (the bridge sends only `cwd` and `mcpServers` on
`session/new` today — [GOOSE-ACP.md](./GOOSE-ACP.md)). Until the probe
passes, treat the rest of this section as the intended target, not a
build instruction.

- Store/emit the step definition as recipe YAML rather than a bespoke
  `steps_json` shape. We get params, subrecipes, and required-extension
  pinning without owning their schema.
- The wrapper stays ours and stays thin: `trigger_kind`/`trigger_config`
  (when it fires) and `approval_policy` (`require_approval` |
  `auto_approve`, [WORKFLOWS.md](./WORKFLOWS.md#approval-policy-the-load-bearing-r6-semantics)).
- Crucially, the recipe must be **driven through our gated ACP loop**, never
  through Goose's own runner (`goose run --recipe`, which approves under
  Goose's own mode and bypasses the seam). We rent the *definition*; we keep
  the *execution path* so every step still hits the approval gate.

This sharpens R3 ("rent the agent") into "rent the recipe too." The open
question in [WORKFLOWS.md](./WORKFLOWS.md) about per-step vs per-workflow
auto-approve maps cleanly onto recipe steps once the step shape is Goose's.

Migration note (once the probe passes): build step 6 already shipped
`steps_json`. This is a format swap, not a rebuild — `run_workflow` keeps
its drive/auto-resolve logic; only what it loads as the step source changes.

## Skills — adopt; this is the missing layer

A Goose **skill** is a reusable instruction set that shapes how the agent
behaves across sessions (hot-reloadable, shareable via the community
marketplace). It is the *behavior* layer — distinct from extensions (the
*capability* layer) and recipes (the *workflow* layer). Lazyboy has no
equivalent: a space's agent behavior today is whatever prompt happens to be
in the run.

This is the clearest gap, and it fits "rent" perfectly because Goose loads
skills natively. Two intended uses:

- **Per-space behavior profile** — a space could carry one or more skills
  ("triage inbound support email this way", "our PR-review checklist") that
  ride into every agent run scoped to it. Reusable, versionable, not
  reinvented.
- **Per-workflow behavior** — a workflow references the skills its recipe
  assumes, the same way it pins required extensions.

Probe first, then build. `goose skills --help` only confirms skill
*listing*, not injection over ACP. So step one is to confirm whether
v1.37.0 accepts skills on the ACP `session/new` path (the way `mcpServers`
is accepted) or only via CLI/desktop config. **Only if that probe passes**
do we add a `skills` reference to the space/workflow model (a list of skill
ids/paths) and pass it on the ACP handshake alongside `mcpServers`. No new
engine, no skill format of our own.

## Scheduler — do NOT rent; build the thin tick

Goose's **scheduler** runs a recipe on a cron, spinning up a *fresh agent
per run* with `schedule_id` session metadata. It is the most tempting
feature to rent and the one we most clearly **cannot**, for the deciding
reason above:

- A Goose-scheduled run uses Goose's own approval mode (`auto`) — it runs
  **around** our `session/request_permission` seam. That is a direct R6
  violation: unattended outside-world actions with no durable approval row.
- It is **not space-bound** and does **not import into our timeline**, so
  "what did this automation do, on whose authority" is unanswerable —
  breaking R1 (SQLite is the source of truth) and the entire audit invariant
  ([WORKFLOWS.md](./WORKFLOWS.md#the-auto-approve-audit-invariant-write-then-resolve-never-bypass)).

The `schedule` trigger and the **stage-C always-on daemon** (the missing
piece in the AI-keystone roadmap — ingress→`dispatch_feed_event` +
schedule/reminder tick) are therefore **trust-layer infrastructure, ours by
necessity**, not a feature to rent. Keep them thin: the tick does nothing
but fire `run_workflow` / `dispatch_feed_event` on a clock, and every run it
starts goes through the same gated drive loop as an interactive run.

Constraint to honour while building it: the daemon spawns/owns timers and
the feed transport — it must stay **out of the mobile-safe crate graph**
(the workspace CLAUDE.md crate-isolation rule; process spawn lives only in
`lazyboy-adapters-host`). Process spawn and the live scheduler live host-side
(`lazyboy-adapters-host` / the host wiring), never in a
`codeless-types`/`codeless-client` dependency path.

(Optional, low priority: Goose's scheduler could still run *ungated,
non-space* maintenance recipes — housekeeping that never touches the
timeline. Not worth wiring for MVP; noted only so the rejection above is
understood as "not for gated space runs," not "never useful.")

## Apps — skip

Goose **MCP Apps** render interactive UI (buttons, forms, visualizations)
**inside Goose Desktop**. They are desktop-shell-coupled, which collides
head-on with two hard rules:

- **R2 (single transport, single client interface)** — the single React UI
  imports only `RpcClient` and never knows which shell it runs in. A
  Goose-Desktop-hosted UI is a parallel surface the shared UI can't reach,
  and an MCP App is exactly the per-shell UI fork R2's single-client-interface
  rule forbids. (The workspace CLAUDE.md states this UI prohibition as its
  own R3; under SCOPE it lives inside R2.)

Lazyboy's own React UI *is* the interactive surface. If a workflow needs a
form or a button, it belongs in the Lazyboy UI over `RpcClient`, not in a
Goose App. Explicitly out of scope.

## Subagents — allow, don't model

Goose **subagents** spawn parallel helpers within a run (code review,
research, batch processing) to keep the main conversation clean. This needs
no Lazyboy concept: it is an internal capability of the rented agent. The
only rule that applies is the existing one — any subagent action that
touches the outside world arrives at the same `session/request_permission`
seam and parks the same durable approval row. We neither expose nor restrict
subagents; we gate their effects like any other tool call.

## Net changes this implies

1. **Workflow format** — probe **resolved NO** (Decision 2). `steps_json`
   stays; no recipe-YAML swap. The gated ACP seam does not accept a recipe.
2. **Skills** — probe **resolved NO** (Decision 1). No `skills` reference on
   space/workflow; `session/new` does not accept skills.
3. **Scheduler** — build the stage-C tick as thin trust-layer infra
   (host-side, mobile-safe crate graph); do **not** call Goose's scheduler
   for gated runs.
4. **Apps** — none; the Lazyboy UI is the surface.
5. **Extensions / subagents** — no change; already rented / internal.

## Settled decisions (probed v1.37.0, 2026-06-14)

Both probe-gated proposals above are **closed: NO** against the pinned
`bin/goose` v1.37.0. The ACP `session/new` wire accepts neither skills nor a
recipe, so neither the recipe-format swap nor a `skills` field is built. The
evidence is from the binary itself (the sandbox cannot run `goose serve`, so
findings are extracted from the binary's emitted serde field literals and
struct-arity markers, plus the CLI `--help` surface).

### Decision 1 — skills are NOT injectable over ACP `session/new`. Field not added.

`session/new` deserializes into `NewSessionRequest`, whose complete field set
is fixed at **four**: `clientCapabilities`, `cwd`, `additionalDirectories`,
`mcpServers` (binary markers `struct NewSessionRequest with 4 elements` and
`struct InitializeRequest with 4 elements ... cwd additionalDirectories`; the
only generated field-accessor symbol on the struct is
`NewSessionRequest11mcp_servers`). `LoadSessionRequest`/`ForkSessionRequest`
(5 fields each) likewise carry no skills field.

`skills` appears in the binary only as (a) a boolean **capability flag** inside
`ClientCapabilities` (clustered with `elicitation`, `nes`,
`positionEncodings`, `fs`) — an advertisement, not a data-bearing param you
populate with skill content — and (b) the standalone `goose skills list`
subcommand. Skills are loaded from disk (`~/.agents/skills/`,
`.agents/skills/`, `builtin://skills/`) as an internal MCP-style client
(`goose::skills::client::SkillsClient: McpClientTrait`), surfaced to the model
as a `load_skill` tool. There is no path to supply them as a `session/new`
param the way `mcpServers`/`cwd` are.

Consequence: the space/workflow model is **left unchanged** — no `skills`
reference is added. Behavior shaping for a space stays the run prompt
(`steps_json`) plus the lazyboy MCP server's space binding. If skills become
ACP-injectable in a later goose, revisit; the seam to extend would be
`client.rs::new_session` params, mirroring `mcp_servers`.

### Decision 2 — a recipe cannot be driven over the gated ACP seam. `steps_json` kept.

`--recipe` (and `--sub-recipe`, `--params`, `--render-recipe`) exist **only on
`goose run`**, goose's own headless runner. The `acp` and `serve` subcommands
expose only `--with-builtin`. The ACP session-request structs carry no
`recipe`/`recipePath`/`recipe_name` field (the only recipe-adjacent params in
the binary are `SummarizeParams.recipe_path` and the internal `Session.recipe`
state column — neither a `session/new` input). There is no way to load a recipe
into an ACP session so its steps surface as `session/request_permission`.

Worse, a recipe carries no approval semantics to rent: the `Recipe` struct's
13 fields (`contact version title description instructions prompt extensions
activities author parameters sub_recipes retry json_schema`) and its nested
`Settings` (`goose_provider goose_model temperature max_turns`) contain **no**
mode/approval field. Approval mode is a **session/config** concept
(`GOOSE_MODE`, persisted as the `sessions.goose_mode` column,
`DEFAULT 'auto'`), guarded by `Approve/SmartApprove modes require an
interactive terminal. Use GooseMode::Auto for headless sessions.` So running a
recipe (`goose run --recipe`) is headless `auto` — exactly the un-gated path
SCOPE.md R6/R3 forbid for space runs.

Consequence: **the format swap is NOT done.** `run_workflow` keeps `steps_json`
as the step source and its existing gated drive + auto-resolve logic
unchanged. Renting the recipe *definition* would buy params/subrecipes only by
either (a) driving steps ourselves over a normal ACP session — i.e. exactly the
`steps_json` drive loop we already own, with a recipe parser bolted on for no
gating benefit — or (b) `goose run --recipe`, which bypasses the gate. Neither
is worth the schema dependency. If a future goose accepts a recipe on
`session/new` that emits `session/request_permission` per step, the proposal in
"Recipes — rent the format" above becomes buildable; until then it stays the
intended target, not a build instruction.

### Downstream open questions 3 and 4 — moot under the decisions above.

Open questions 3 (recipe retry/validation re-entering the gate) and 4 (per-step
vs per-workflow auto-approve once steps are Goose's) both presupposed adopting
the recipe format. With Decision 2 negative they do not arise: retries and
approval granularity are governed by our own `steps_json` drive loop and the
per-workflow `approval_policy` (WORKFLOWS.md), not by a recipe. The per-step
auto-approve granularity question stays open in WORKFLOWS.md on its own terms
(our checkpoint shape), independent of Goose.

## Original open questions (resolved above)

1. **Skills on the ACP seam** — **resolved NO** (Decision 1).
2. **Recipe execution boundary** — **resolved NO** (Decision 2).
3. **Recipe retry/validation** — **moot** (depends on Decision 2).
4. **Per-step vs per-workflow auto-approve** — **moot as a recipe question**;
   remains open in WORKFLOWS.md as a `steps_json`-checkpoint question.
