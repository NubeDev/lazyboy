# Zenoh sync fabric

How the team layer replicates a node's timeline (SCOPE.md "Zenoh sync
fabric", build step 5). This is the spec for the local event model
(`outbox_events`), its ordering scheme, the mutable-row merge rule that
resolves SCOPE.md Open Question 1, the peer-vs-broker topology, and the
integration checklist that gates turning Zenoh on.

SCOPE.md's hard rule holds: **no Zenoh until the local event model is
stable.** The outbox writer (`lazyboy-store`) is the solid, well-tested
foundation; the Zenoh networking (`lazyboy-sync`, feature-gated) is a
thin layer over it.

## The local event model: the outbox

Every state change a node makes is appended to `outbox_events`
(0002_domain.sql). SQLite stays the source of truth (R1); the outbox is
only the replication boundary. A row is:

```
outbox_events(id, aggregate, aggregate_id, event_json, seq, created_at, synced_at)
```

- `aggregate` / `aggregate_id` — what changed (`message`/`m1`,
  `task`/`t1`, ...).
- `event_json` — the serialized change a peer applies (op + the row
  fields it needs).
- `seq` — per-aggregate monotonic order (see below).
- `created_at` — when the change happened; doubles as the LWW clock.
- `synced_at` — `NULL` until a publisher has shipped the row; the
  unsynced rows are the replication queue.

Repo verbs (`lazyboy_store::repo::outbox`):

- `append(NewOutboxEvent)` — enqueue, allocating the next per-aggregate
  `seq`.
- `record(aggregate, aggregate_id, event_json)` — thin wrapper a
  mutation verb calls in one line.
- `unsynced()` — the queue: rows with `synced_at IS NULL`, ordered
  `created_at, seq`.
- `mark_synced(id, ts)` — acknowledge a shipped row.

## Seq / ordering scheme

`seq` is allocated as `max(seq)+1` **scoped to the aggregate**, computed
and inserted inside one transaction (`repo/outbox/append.rs`). The
transaction is load-bearing: two concurrent appends that both read the
same `max` would pick the same `seq`, and `UNIQUE(aggregate, seq)` would
reject the second as a constraint error instead of serializing them.
Holding the SELECT...INSERT in one transaction gives each a distinct,
gapless seq.

Seq is **per-aggregate, not global.** A peer reconstructs each
aggregate's order independently; an append-only aggregate's union merge
needs no global clock, and a mutable aggregate's LWW uses `created_at`
with `seq` only as a tie-break.

## Merge rule — resolution of Open Question 1

SCOPE.md Open Question 1 ("Last-writer-wins vs. a small CRDT for task
state and approval status") is **resolved for MVP as last-writer-wins.**

- **Append-only aggregates (messages) union-merge.** A message is never
  mutated; a correction is a new message. Inbound apply is an idempotent
  insert keyed by the originating node's row id (`INSERT OR IGNORE`), so
  a redelivery is a no-op and concurrent peers converge by union. These
  rows never enter the LWW path.
- **Mutable aggregates (task state, approval status) use LWW.** The
  winner is the event with the greater `created_at`; a same-millisecond
  tie across two nodes breaks deterministically on the higher
  per-aggregate `seq`, so every node picks the same winner with no
  coordination. The decision is the pure function
  `lazyboy_sync::merge::incoming_wins` / `apply::decide`; an equal key
  does not overwrite, making re-delivery idempotent.

Rationale: task/approval state for a single initiative is low-contention
and last-intent-wins is the user's mental model ("whoever last set it
wins"). A CRDT buys conflict-free convergence we do not need at MVP
contention levels and costs per-field metadata and a harder mental
model. LWW is revisited only if a concrete lost-update bug appears under
real team use; the merge decision is isolated in one pure, tested
function so swapping it is bounded.

## Topology: peer vs broker

SCOPE.md makes topology a configuration choice, not two code paths
(`lazyboy_sync::config`):

- `Topology::Peer` — direct peer-to-peer mesh for a small flat team.
- `Topology::Client { endpoints }` — dial a router/broker hub for a
  larger org.

Both ride the identical event model. The workspace name scopes every
Zenoh key — `lazyboy/{workspace}/{aggregate}/{aggregate_id}` — so
distinct workspaces never cross and a peer can subscribe to a whole
workspace with `lazyboy/{workspace}/**`.

## Publisher / subscriber

`lazyboy-sync` (feature `zenoh`, default off):

- **Publisher** (`session::publish_pending`) drains `outbox::unsynced`,
  maps each row to a `SyncEvent` on its key (`drain::to_publication`,
  pure), `put`s it, and only then `mark_synced`s it — a transport
  failure leaves the event queued for the next pass rather than dropping
  it.
- **Subscriber** (`session::run_subscriber`) subscribes to
  `lazyboy/{workspace}/*/**`, deserializes each inbound `SyncEvent`, and
  applies it (`apply::apply`): append-only -> insert-if-absent, mutable
  -> LWW overwrite or skip. Inbound writes deliberately bypass
  `outbox::record` (`inbound.rs`) so an applied remote change does not
  re-enter this node's outbox and echo back.

## Feature gating

`zenoh` is heavy networking and a wide dependency tree, so it is gated
behind the `zenoh` feature (default off) on `lazyboy-sync`. Crate
direction (SCOPE cross-cutting, CLAUDE R1) is preserved: zenoh lives
only in this host-side crate, never in `lazyboy-types` or
`lazyboy-store` core types.

- `cargo build --workspace` and `cargo test --workspace` are green
  **without** zenoh and need no network.
- The always-compiled logic is pure and tested without the feature: the
  LWW merge decision, the apply decision, the outbox-to-wire mapping,
  and the event serde round-trip (`tests/pure_test.rs`).
- The live-session paths (`session`, `inbound`) compile and run only
  under `--features zenoh`; any test needing a live session is gated the
  same way, so default `cargo test` never touches the network.

## Integration checklist — wiring `outbox::append`

The outbox only replicates what mutation sites record into it. A
mutation that is not wired is invisible to peers. The gate before
turning Zenoh on is: **every state change that should replicate appends
an outbox event.**

Wired in this change (store-level verbs, additive one-liners):

- [x] `repo::message::append` — `aggregate = "message"`, append-only
      (union merge). Records the full row for idempotent remote insert.
- [x] `repo::task::set_state` — `aggregate = "task"`, mutable (LWW).
      Records `state` + `updated_at` as the LWW clock.

Still to wire (left as checklist; these live in core/bridge verbs that
other work owns — wire each with a single `outbox::record` call and add
its inbound arm in `lazyboy_sync::inbound`):

- [ ] `repo::approval::resolve` — `aggregate = "approval"`, mutable
      (LWW on `resolved_at`). The trust-layer row; replicating it is how
      a teammate's approve/deny reaches other nodes.
- [ ] `repo::approval::request` — append of a pending approval row.
- [ ] `repo::run::set_status` — `aggregate = "agent_run"`, mutable (LWW
      on the status-change time).
- [ ] `repo::run::append_event` — `aggregate = "agent_run_event"`,
      append-only (union by `(run, seq)`).
- [ ] `repo::artifact::create`, `repo::decision::record`,
      `repo::reminder::*`, `repo::calendar::upsert` — as the durable
      memory (build step 4) is brought under sync.
- [ ] ingress (`repo::ingress`) — replicate the mapped message, not the
      raw external event; the dedupe boundary stays node-local.

Until the rows a feature depends on are wired and the local model is
exercised under restart, that feature's Zenoh sync stays off.
