# Workflows, automation, and membership (build step 6)

Implements [SCOPE.md "Workflows and automation (build step 6)"](./SCOPE.md#workflows-and-automation-build-step-6)
and [SCOPE.md "Feeds, membership, and visibility (post-step-3)"](./SCOPE.md#feeds-membership-and-visibility-post-step-3).
Terms (workflow, automation, workflow agent, approval policy, user,
group, space membership, feed, feed visibility) are defined in
[TERMINOLOGY.md](./TERMINOLOGY.md).

## Trigger model

A **workflow** is a saved, triggerable agent run: a `workflows` row
carrying a name, a trigger (`trigger_kind` `feed` or `schedule`, plus
`trigger_config_json`), an approval policy, and `steps_json` (the prompt
and any inter-step approval checkpoints). A workflow is created
`disabled` and inert. Enabling it (`status = enabled`) arms its trigger;
SCOPE.md calls an enabled workflow an **automation**. Disable disarms it
again. Enable/disable is the only mutation the trigger state needs
(`repo::workflow::set_status`).

Each firing opens an agent run and records a `workflow_runs` row linking
the workflow to the `agent_run` it created, so "what did this automation
do" is answerable from the timeline.

## Approval policy (the load-bearing R6 semantics)

Every workflow carries an `approval_policy`, the user's per-workflow
choice. Implemented in `lazyboy-core` `run_workflow.rs`:

- **`require_approval`** (default): identical to an interactive run. The
  existing drive loop parks the first outside-world step as a `pending`
  `approvals` row and returns `AwaitingApproval`; a human resolves it
  later. No special path.

- **`auto_approve`**: the single sanctioned R6 exception, chosen per
  workflow by a human — never a global gate-off switch.

### The auto-approve audit invariant (write-then-resolve, never bypass)

When a workflow is `auto_approve`, the gate is **not** skipped. The
implementation reuses the ordinary approval machinery:

1. The drive loop hits the tool request and, exactly as for any run,
   writes the durable `approvals` row through `import_update` ->
   `approval::request`. **The row exists before anything is answered.**
   This is the audit invariant: "what did the agent do and on whose
   authority" stays answerable.
2. Only then does `run_workflow` find that just-parked row
   (`approval::pending_for_run`) and auto-resolve it through the normal
   `resolve_approval` path — status `approved`, `resolved_by` = the
   workflow's agent principal — which answers Goose and continues
   driving.
3. The loop repeats for each checkpoint until the run ends.

So the sequence is **write-then-resolve**, never write-skip. A test
(`auto_approve_workflow_writes_then_resolves_and_completes`) asserts the
row still exists with status `approved` and `resolved_by` set, and that
the run reached `TurnEnded`.

## Workflow agent (selection model)

The **workflow agent** watches feeds, picks which workflow to fire, and
sequences the steps. It DRIVES Goose — every step is still a Goose tool
call (R3); it does not replace the agent loop.

For MVP it is a thin selection-and-invocation function, not a live
loop: `Engine::dispatch_feed_event(&FeedEvent)`. Given a feed/ingress
event, it selects the workspace's workflows that are (a) enabled — an
automation — (b) feed-triggered, and (c) whose `trigger_config_json`
matches the event's match key, and calls `run_workflow` for each. MVP
match is equality on the trigger config; richer pattern matching is a
later concern.

### Live-trigger-daemon integration point

The live scheduler/feed-watcher daemon that arms triggers, polls or
receives webhooks, and delivers `FeedEvent`s is the **host-side
integration point**, deliberately out of this layer. It builds on feeds
(build step 3) and the team layer (build step 5), both now present. The
core exposes the pure selection + invocation; the host wires the timer
and the feed transport to it.

## Membership and feed visibility — modeled, not enforced (R4)

The user/group, space-membership, and feed-visibility tables
(`groups`, `group_members`, `space_memberships`, `feed_visibility`) and
their repo verbs (`repo::membership::*`) and HTTP endpoints (`POST
/groups`, `POST /groups/{id}/members`, `POST /spaces/{id}/members`,
`POST /feeds/{integration_id}/visibility`) **model** the first structure
past single-tenancy.

Per **R4**, none of this is wired into the MVP trust gate. The bearer
token still authorises browser, CLI, and mobile clients identically;
anyone in the workspace can approve any approval; every feed is visible
to the single trust boundary. The membership model is written so the
structure exists and can be populated, but it does not change who may do
what until it is promoted past MVP (Phase 7 OIDC in SCOPE.md). Endpoint
and repo comments flag this with a pointer back here.
