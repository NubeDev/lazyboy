# Lazyboy IDEA-V3 — Runtime Bake-off Spike

Addendum to [IDEA-V3](./IDEA-V3.md). V3 names Goose as the runtime but defers
the one question that "decides the bridge design" (V3 §Approvals) to a spike
run *after* the choice is made. This doc inverts that: it specs a head-to-head
spike that runs the same test against **Goose** and **Awaken**, and lets the
result name the winner.

Run this before any product code. Nothing in step 2+ of the V3 build order
starts until this exits green for at least one runtime.

Candidates are runtimes that can be driven headless and externally own an
approval. Goose and Awaken qualify. Zylos does not (autonomous-by-design, no
HITL, tmux daemon) — see [IDEA-V3](./IDEA-V3.md) §Prior art; it is excluded
from the bake-off, not a fourth column here.

## Why a bake-off, not a confirmation

Lazyboy owns exactly one hard thing: **approval as a durable row the agent
waits on, in the timeline** (V3 line 107). Everything else — agent loop, tools,
sandbox — is rented and commoditized. So the runtime must be chosen on the axis
where Lazyboy is opinionated, not on generic runtime maturity.

- Goose wins on maturity, tooling (70+ MCP exts), sandbox, Block backing.
- Awaken wins on architectural fit: mailbox suspension *is* durable-pause-with-
  external-resume, which is Lazyboy's core primitive.

If Goose surfaces approvals cleanly over its API, it wins outright — its other
advantages are real and Awaken's youth is a maintenance risk. If it does not,
Awaken's fit earns the seat. We do not know which is true. The spike decides.

## The one gate that matters

A runtime passes only if an **external process** (our throwaway harness,
standing in for Lazyboy core) can own the full approval round-trip without
reaching into the runtime's internals:

```text
1. Harness starts the runtime headless and opens one session.
2. Harness sends a prompt whose only path to completion is a gated tool
   (e.g. "create the file ./SPIKE_OK.txt with the text PASS").
3. Runtime emits a tool-request event over its API/ACP, naming the tool
   and its input, BEFORE executing it.
4. Harness persists an `approvals` row (SQLite) and does NOT answer yet.
5. Harness kills the runtime process (crash simulation).
6. Harness restarts the runtime and rebinds to the same session.
7. Harness writes the approval decision (approve) over the API.
8. Runtime executes the tool in that same session and runs to completion.
9. ./SPIKE_OK.txt exists with text PASS.
```

Steps 5–6 are the load-bearing ones. Anyone can stream a tool event; the
question is whether the pause is **durable state the runtime can resume**, or
an in-memory promise that dies with the process. Awaken's mailbox model claims
the former by design. Goose must be shown to do it.

### Pass / fail, no partial credit

A runtime **passes** only if every line below is true. Log the actual observed
behaviour for each, not just a checkmark.

| # | Assertion | Fail mode if absent |
| --- | --- | --- |
| G1 | Tool request is surfaced over the public API/ACP, naming tool + input, before execution | Approval UX is welded to the runtime's own CLI/desktop; can't externalize |
| G2 | The run blocks on the request — does not auto-proceed, does not time out within 60s | Approval is advisory, not a gate; trust boundary unenforceable |
| G3 | The decision is accepted over the public API by an unrelated process | Resume requires being the original in-proc caller; no sidecar seam |
| G4 | The session survives a runtime process restart and resumes the *same* run | Pause is in-memory; a crash mid-approval loses the run (violates V3 thesis) |
| G5 | None of G1–G4 required forking, patching, or importing the runtime's internal crates | Violates the V3 "never fork the big thing" rule |

G4 is the discriminator most likely to separate the two runtimes. Spend the
most time there.

## What to build (throwaway, ~1 file per runtime)

A single Rust binary per runtime, or one binary behind a `--runtime` flag.
No product code, no shared abstraction yet — that's the point of a spike.

```text
spike/
  src/main.rs        -- arg-parses --runtime {goose,awaken}, runs the 9 steps
  src/goose.rs       -- start goosed, open session, the API calls for G1-G4
  src/awaken.rs      -- start awaken server mode, same surface
  approvals.sqlite   -- one table, written at step 4, read at step 7
```

`approvals` table — the minimal shape, deliberately the V2 column set so the
spike's storage is forward-compatible with the real product:

```text
approvals(
  id, agent_run_id, runtime_session_id,
  tool_name, tool_input_json,
  status,            -- pending | approved | denied
  requested_at, resolved_at
)
```

Keep it dumb. No migrations framework, no repository layer. If the spike needs
those it has stopped being a spike.

## Pre-spike: verify the table claims (1–2 hrs, do first)

V3's comparison table (lines 32–40) asserts facts with no sources. Confirm or
correct these before spiking — a wrong license or a missing server mode kills a
candidate before any code runs:

- [ ] **Goose license** — confirm Apache-2.0 in `block/goose` LICENSE.
- [ ] **Goose `goosed`** — confirm a headless daemon exists and its API surface
      (REST? WS? both?). Find the tool-approval message in its API docs/source.
- [ ] **Awaken license** — V3's own table says MIT/Apache in one cell and calls
      license a non-issue; confirm the actual SPDX in the repo.
- [ ] **Awaken server mode** — confirm it exists and that mailbox suspension is
      reachable over it, not just from an embedded library call.
- [ ] **ACP reality** — confirm both speak ACP and whether approval semantics
      ride ACP or a runtime-specific side channel. V3 leans on ACP for
      swappability (line 44); if approvals are *not* in ACP, that claim weakens.
- [ ] **ZeroClaw** — V2 already inspected it at commit
      `ffb02755`; if it's still a live candidate, confirm its license (V3 marks
      it "unknown (risk)") rather than carrying the unknown forward.

Record findings inline in this file or a sibling note, with commit SHAs and
dates, matching the rigor V2 used for Lukan/ZeroClaw.

### Pre-spike findings (2026-06-13)

Source-level review of both repos. These are code/doc-confirmed, not the
empirical 9-step run — that still has to happen (see "Status" below) — but the
evidence is strong enough to predict the gates and it already moves the V3
decision.

**Goose** — `block/goose`, `main` @ `f40d56fe09cd51f8fc33e14ae276f8c1b75e6439`,
release v1.37.0 (2026-06-03).

- [x] **License** — Apache-2.0. Confirmed in `/LICENSE` (raw on `main`).
- [x] **`goosed`** — exists (`goose-server` crate, Axum). Surface is REST + SSE:
      `POST /reply` streams a `MessageEvent` enum over SSE. A separate
      ACP-over-HTTP transport (JSON-RPC 2.0 + SSE, WS upgrade) is in progress,
      not the current stable surface.
- [x] **Tool request over the API (G1/G3)** — real and external-drivable. The
      "agent wants tool X" event arrives in the `/reply` SSE stream as
      `MessageContent::ActionRequired` (tool-confirmation, carrying an id); the
      decision goes back via `POST /action-required/tool-confirmation`
      (`crates/goose-server/src/routes/action_required.rs`). Permission modes:
      Chat / Auto / Approve / SmartApprove. **Caveat:** no endpoint to *list*
      pending confirmations — recovery depends on the client having captured the
      SSE event.
- [x] **ACP** — yes (`goose-acp-server`, powers Zed/JetBrains). Approval rides
      native ACP `request_permission`. Consolidation of `goosed` onto
      ACP-over-HTTP is open and in progress (Issue #6642, since 2026-01-22) — the
      REST+SSE surface is actively shifting.
- [!] **Durable pause across restart (G4)** — **FAIL.** Pending approval state is
      an in-memory `tokio::oneshot`, not persisted:
      `ToolConfirmationRouter { pending: Mutex<HashMap<String, oneshot::Sender<…>>> }`
      (`crates/goose/src/agents/tool_confirmation_router.rs`) and the same shape
      in `crates/goose/src/action_required_manager.rs`. Session *history*
      (messages, tool calls/results) persists to SQLite (`sessions.db`, sqlx,
      v1.10.0+), but the suspended-awaiting-decision continuation does not. Kill
      `goosed` mid-approval and a later POST with the old id hits the
      "No task waiting for confirmation" path; there is no list-pending endpoint
      to even rediscover it. This is exactly the gate the spike said to spend the
      most time on (line 70), and it is the gate V3's whole thesis rests on.

**Awaken** — `AwakenWorks/awaken`, `main` @
`2b0d375004ec8723cf40d8a59c0bb486ebce57e0`, release v0.6.0 (created 2026-02-22).

- [x] **License** — MIT OR Apache-2.0 (`LICENSE-MIT` + `LICENSE-APACHE`).
      GitHub's `NOASSERTION`/"Other" flag is just dual-license misdetection.
- [x] **Server mode** — real headless HTTP/SSE server (`crates/awaken-server`),
      distinct from library mode. Protocol adapters (`ai-sdk`, `ag-ui`, `a2a`,
      `mcp`) plus run/thread control (`POST /v1/runs/:id/inputs`,
      `/v1/runs/:id/cancel`, `GET|POST /v1/threads/:id/mailbox`).
- [x] **Mailbox suspension over the server (the reason it's a candidate)** —
      reachable out-of-process: `POST /v1/runs/:id/decision`
      (`{ toolCallId, action: "resume"|"cancel", result }`) and
      `GET /v1/threads/:id/mailbox` to peek pending. Out-of-process HITL is real.
- [x] **Durable pause across restart (G4)** — **PASS, conditional on backend.**
      `SqliteMailboxStore` (`crates/awaken-stores/src/sqlite_mailbox/`, real
      rusqlite, `run_dispatches` + `thread_dispatch_epochs` tables) persists
      suspended state; run checkpoints persist at each `StepEnd` incl. suspended
      tool-call states; startup recovery replays `RunStatus::Waiting` runs
      (ADR-0019 `recover()`). **In-memory backend is process-local and would fail
      G4** — the spike must select SQLite/Postgres.
- [x] **ACP** — yes, via the official `agent-client-protocol` SDK over **stdio
      only** (not an HTTP route). Approval rides native ACP `request_permission`,
      mapped to a `ToolCallResume`. The durable network resume path is the HTTP
      `…/decision` endpoint, anchored on the shared run/checkpoint store.

**ZeroClaw** — `zeroclaw-labs/zeroclaw`, `master` @ v0.8.0.

- [x] License resolved: **MIT OR Apache-2.0** (split `LICENSE-MIT`/`LICENSE-APACHE`
      tripped naive detectors — that was the "unknown (risk)" cause, not a real
      gap). Stays **excluded** per V3 §Prior art (no HITL primitive); recorded
      only to close the carried-forward unknown.

**What this means for the decision.** On source evidence, the bake-off is not
close on the one axis the spike says decides it (G4): Goose fails it, Awaken
passes it. Per the Decision rule below, "if only Awaken passes: pick Awaken."
V3 line 24 ("Chosen runtime: Goose") is contradicted by the code. The empirical
9-step run is what makes this binding; see Status.

**Status / next.** Neither runtime is installed in this environment
(`cargo`/`rustc` 1.91.1 present, network up). The empirical 9-step harness is the
remaining work and requires building/installing `goosed` and `awaken` first.

## Decision rule

```text
if exactly one runtime passes all of G1-G5:  pick it. Done.
if both pass:                                 pick Goose (maturity + tooling
                                              break the tie; Awaken's fit is
                                              no longer load-bearing).
if only Awaken passes:                        pick Awaken; its core primitive
                                              is the product thesis.
if neither passes:                            the sidecar boundary is wrong for
                                              approvals. Re-open IDEA-V3
                                              §Integration — the seam, not the
                                              runtime, is the problem. Do NOT
                                              fall back to forking.
```

The last branch is the one V3 doesn't consider and the one most worth naming up
front: if no headless runtime can externalize a durable approval, the
"rent the big thing" architecture has a hole, and that's a finding about the
*architecture*, not a reason to fork.

## Out of scope for this spike

- Tool variety, MCP extensions, sandbox quality — all real, none decide the
  seam. Evaluate after the runtime is chosen.
- Streaming fidelity, token costs, model routing — product polish, later.
- Any Lazyboy domain table other than `approvals` — the spike proves the seam,
  not the schema.

## Exit

Green when one runtime passes G1–G5 and the table claims are confirmed. At that
point IDEA-V3 line 24 ("Chosen runtime: Goose") gets edited to match the actual
result — whichever way it lands — and V3 step 2 (product core) can start.
