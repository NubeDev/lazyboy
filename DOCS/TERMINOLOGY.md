# Lazyboy — Terminology

The shared vocabulary for Lazyboy. Where a term names something
[`SCOPE.md`](./SCOPE.md) already defines, this doc points at it rather than
redefining; where a term is new, it is flagged **NEW** and the open
questions are called out. SCOPE remains the canonical spec — if this doc
and SCOPE ever diverge on an existing concept, SCOPE wins.

Naming principle threaded through all of it: **rent Goose for the agent,
its tools, and its sandbox; we build only the team-and-trust layer**
(spaces, feeds, the inbox, the approval queue, visibility). Every "tool"
is a Goose MCP extension, never our code.

## Core terms

### space
A Slack-channel-shaped container for one idea or initiative
(`#new-pricing-page`). It is the unit of everything: messages, tasks,
feeds, agent runs, and approvals all belong to exactly one space.
- Maps to: the existing **space** in SCOPE ("space-as-worker model").
- Holds: a timeline of messages, the tasks worked in it, the feeds wired
  into it, and the agent runs scoped to it.

### feed
An input **source bound to a space** — Slack, email, GitHub, calendar.
External events on a feed arrive in the space as messages.
- Maps to: SCOPE's **ingress** layer. "Feed" is the user-facing name for
  an ingress source attached to a space; "ingress" stays the name for the
  internal mechanism that turns external events into messages.
- A space can have several feeds (e.g. a Slack feed and a private email
  feed). See [feed visibility](#feed-visibility) for who sees which.

### tool
An external capability the agent can use — GitHub, shell, files, http,
browser.
- Maps to: a **Goose MCP extension**. We write zero tools. "Add the
  GitHub tool to this space" means the agent run uses Goose's `github`
  MCP extension; it does not mean Lazyboy code.
- Every tool action that touches the outside world is gated by an
  [approval](#approval--approval-queue).

### task
A unit of work in a space — created from talk, run by the agent.
- Maps to: SCOPE's **task** (`open | running | blocked_on_approval |
  done | cancelled`).

### agent run
One scoped execution of Goose against a space, driving a task. Sees the
space's context, drops its output back into the space timeline.
- Maps to: SCOPE's **agent_run**.

### approval / approval queue
A single agent action waiting on a human's yes/no, and the per-user view
of all such pending actions.
- The individual approval maps to: SCOPE's **approvals** row (the durable
  trust layer — a row in our SQLite the agent waits on, surviving a
  crash).
- **approval queue** is the user-facing view: "things waiting for my
  decision." **NEW as a view** (the underlying rows exist; the
  cross-space per-user aggregation does not yet).
- Naming note: this is what was loosely called "outbox" in conversation.
  We do **not** call it outbox — see [outbox](#outbox-reserved) below.

## Activity views (NEW)

These are per-user views over existing data, not new storage of truth.
Both are **NEW**; neither exists today.

### inbox
A user's attention view: new activity and new messages across the spaces
they belong to. The "what needs me" list.
- **NEW.** Requires per-user read/seen state, which does not exist yet
  (the timeline is stored, but nothing tracks what a given user has
  seen).
- Open question: confirm inbox is strictly per-user (each member has
  their own), not a shared per-space counter.

### approval queue
See [approval / approval queue](#approval--approval-queue). The companion
to the inbox: inbox is "new things to read," approval queue is "decisions
to make."

### outbox (RESERVED — not the approval queue)
`outbox` is **already taken** in SCOPE for the **sync/replication
boundary**: every state change is written to `outbox_events`, which the
(later) Zenoh team-sync layer ships to replicate the timeline. It is
infrastructure, not a user-facing list.
- Do not use "outbox" for the approval queue. The user-facing "waiting on
  me" list is the [approval queue](#approval--approval-queue).

## Membership and access (NEW)

All **NEW**. Today Lazyboy is single-tenant: one workspace, one trust
boundary, every member sees everything. The terms below add structure
inside that boundary and will need new tables and access rules.

### user
A human member, identified in the system.
- Maps to: an **identity** of kind `human` in SCOPE. "User" is the
  product word; "identity" is the storage/attribution word (identities
  also cover agent and integration principals).

### group
A named set of users, addable to a space as a unit.
- **NEW.** No grouping of identities exists yet.

### space membership
Adding a user or a group to a space, granting them access to it.
- **NEW.** Spaces have no membership model today; access is implicit
  (whole workspace).

### feed visibility
Per space, controlling which users/groups can see a given feed — e.g. a
private email feed only you see. Two actions:
- **share an item**: manually surface one message from a hidden feed to
  chosen users/groups.
- **auto-share a feed**: make a whole feed visible to chosen
  users/groups by default.
- **NEW**, and the most significant addition: today everyone in the
  workspace sees everything in a space. This introduces per-feed,
  per-user/group access control *inside* a space (e.g. an email lands on
  a space's private feed; you decide whether to share it).

## Modes

### workshop
A collaborative mode for developing an idea in a space before (or
alongside) turning it into tasks — e.g. working up a new marketing idea
with the team and the agent.
- Partial overlap with SCOPE's existing "talk -> tasks" flow. **NEW** as a
  named, distinct activity; needs definition of how a workshop differs
  from ordinary space talk (is it a phase, a sub-thread, a state?).

## Workflows and automation (NEW)

All **NEW**. None of this exists yet. A workflow does not introduce a new
engine — it is a *saved, triggerable* [agent run](#agent-run) with a
trigger and an approval policy. Goose still does every actual step; the
new parts are the trigger, the sequencing, and the per-workflow choice of
whether steps wait on a human.

### trigger
What starts a workflow without someone typing a prompt. Two kinds:
- **feed trigger** — an event on a space [feed](#feed) (email arrives, PR
  opened, message matches a rule).
- **schedule trigger** — a clock (nightly, weekly), cron-shaped.
- **NEW.** Today a run only starts from an explicit prompt; there is no
  trigger mechanism.

### workflow
A saved definition of *trigger -> agent run (with tools) -> result back in
the space*, optionally multiple steps. Lives in a space, reusable.
- **NEW.** Maps onto existing pieces: the run is an [agent run](#agent-run),
  each outside-world step is gated (or not) per the
  [approval policy](#approval-policy), results land as messages/artifacts.
- A multi-step workflow can place an approval checkpoint between steps
  (e.g. draft -> approve -> open PR -> approve -> merge).

### automation
A workflow that is **enabled and live** — its trigger is armed and it
fires on its own. "Workflow" is the definition; "automation" is a
workflow turned on for a space.
- **NEW.** Distinction matters for the UI: you author/edit a workflow,
  then enable it as an automation (and can disable it).

### workflow agent
A run type whose job is **orchestration**, not the work itself: it watches
feeds, decides which workflow to fire, sequences the steps, and parks at
each [approval](#approval--approval-queue). It drives Goose; it does not
replace it (keeps the "rent Goose" rule — every actual step is a Goose
tool call).
- **NEW.** Open question: is this a distinct process/principal, or just a
  mode of the existing engine driving a saved step list?

### approval policy
The per-workflow user choice for how its steps clear: **auto-approve** or
**require approval**.
- **require approval** (default, the safe one): every outside-world action
  the workflow takes parks as a pending [approval](#approval--approval-queue),
  exactly like an interactive run. Nothing acts unattended.
- **auto-approve**: the user opts a workflow into acting without waiting —
  its steps resolve automatically instead of parking. The trust primitive
  is unchanged; an auto-approve workflow simply auto-resolves its own
  `approvals` rows (still recorded in the timeline for audit).
- **NEW.** This is a policy layer on top of the existing `approvals`
  model, not a new trust mechanism. The choice is the user's, per
  workflow.
- Open question: is auto-approve all-or-nothing per workflow, or per step
  (e.g. auto-approve the read-only summary step, gate the PR-merge step)?

## Open questions

1. **inbox/approval-queue scope** — per-user (each member has their own)
   confirmed? Assumed yes.
2. **workshop** — what makes it distinct from normal space talk: a space
   state, a thread type, or just a UI framing?
3. **feed visibility storage** — visibility is per (feed, space,
   user/group); the access model and its tables are undesigned.
4. **groups vs. single-tenant** — membership/groups/visibility all push
   past the MVP single-tenant trust boundary (SCOPE R4). These are
   post-MVP unless we promote them; flag intended build-order placement.
