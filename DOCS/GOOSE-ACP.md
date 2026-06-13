# Goose ACP seam — verified contract (goose 1.37.0)

This file records what `goose serve` actually exposes, probed against
the pinned `bin/goose` (v1.37.0). It supersedes the API details in
`SCOPE.md` "Approvals and the crash-resume seam", which described an
older `goosed` REST + `/reply` SSE shape that this Goose no longer
ships. SCOPE's *architecture* (durable approval row is ours, Goose
only resumes the run) is unchanged; only the wire seam below is
authoritative for bridge code.

## Transport

`goose serve --host 127.0.0.1 --port 3284` exposes the **Agent Client
Protocol (ACP)** over one path:

- `POST /acp` — JSON-RPC 2.0 request/response. One call, one reply.
- `GET  /acp` with a WebSocket upgrade (`101 Switching Protocols`) —
  the streaming channel carrying agent-initiated JSON-RPC: `session/update`
  notifications and `session/request_permission` requests.

Both responses carry correlation headers:
`acp-connection-id` and (once a session exists) `acp-session-id`.

### Connection model (probed v1.37.0, 2026-06-13)

The WebSocket is not optional streaming bolted onto a request/response
API — it *is* the connection. The load-bearing details, all verified
against `bin/goose`:

- The `acp-connection-id` originates on the **WebSocket upgrade**
  (`101`) response. Open the WS first; every `POST /acp` after that must
  carry that id in an `acp-connection-id` header or goose answers
  `400 Bad Request: Acp-Connection-Id header required`.
- `initialize` answers **synchronously** in the POST body (`200`). This
  is the *exception*: a transport that assumes every reply comes over the
  WS hangs on connect, since the `initialize` oneshot never fires. The
  host client reads a non-empty `200` body inline and only falls back to
  the WS for `202` acknowledgements.
- `session/new` answers **`202 Accepted` with an empty body**. Its
  JSON-RPC result — `{ "result": { "sessionId": "20260613_2", "modes":
  {...} } }` — arrives **over the WebSocket**, interleaved with
  `session/update` notifications, correlated by the request `id`.
- So a POST is fire-and-acknowledge; the real reply (for anything but
  `initialize`) is read off the WS by matching JSON-RPC `id`. A blocking
  client that waits on the POST body for a session id would hang.
- `session/new` reports the available permission modes; `approve` and
  `smart_approve` are the ones that produce the `session/request_permission`
  gate Lazyboy intercepts. `auto` approves silently and must not be used
  for gated runs.

### Turn end is the prompt response, not a notification

The end of a turn is the **`session/prompt` response** (carrying a
`stopReason`), correlated by the prompt request's id over the WS — not a
distinct `session/update`. So `session/prompt` must be fired
fire-and-return: if a client awaited that response it would block through
the whole turn (and indefinitely through an approval pause, since goose
withholds the response until the gated tool is answered). The host
transport (`lazyboy-adapters-host`) instead records the prompt id, lets
the driver pull streamed updates, and converts the eventual prompt
response into the bridge's `Update::TurnEnded` on the session queue.
`stopReason` `end_turn`/`max_tokens` is a clean stop; anything else is a
non-clean end.

### Answering a permission request

The reply to `session/request_permission` is a JSON-RPC **response**
(not a new request) echoing the request id, sent back over the same WS:
`{ "outcome": { "outcome": "selected" | "cancelled" } }`. `selected`
releases the gated tool, `cancelled` denies it. The exact `outcome`
payload still needs confirmation against a live gated run (blocked here
by the sandbox refusing to launch `goose serve`); the transport shape
above is verified.

## Test strategy

The host transport (`lazyboy-adapters-host`) is verified at three levels
without the real binary, since the sandbox will not run `goose serve`:

- **Unit** — `wire.rs` decode and `conn.rs` demux/dispatch (prompt
  response → `TurnEnded`, drop-drain) against hand-built frames.
- **Transport integration** — `GooseServeClient` driven against an
  in-process fake (`tests/support/fake_acp_server.rs`, axum) reproducing
  this contract: WS-minted connection id, `400` without it, `initialize`
  inline, `session/new`/`prompt` over the WS, and the gated approval
  round-trip (permission request → WS answer → resume).
- **Full stack** — the engine (`start_run` → durable approval →
  `resolve_approval`) run end to end against the gated fake, mirroring the
  `FakeGoose` slice test but exercising the live wire path.

`tests/live_handshake_test.rs` is the same shape against the real binary,
gated to skip when `bin/goose` is absent — the remaining step in an
environment that can run `goose serve` with a configured provider.

## Handshake

`initialize` (verified response, v1.37.0):

```json
{ "protocolVersion": 1,
  "agentCapabilities": {
    "loadSession": true,
    "promptCapabilities": { "image": true, "embeddedContext": true },
    "sessionCapabilities": { "list": {}, "close": {} } },
  "authMethods": [ { "id": "goose-provider", "name": "Configure Provider" } ] }
```

`loadSession: true` is load-bearing for Lazyboy: it is the native
`session/load` the crash-resume reconcile (SCOPE step 2→4) drives, so
the re-drive is a supported path, not a workaround.

## The approval seam

A tool that needs confirmation arrives as an agent->client JSON-RPC
**request** `session/request_permission` on the WebSocket, naming the
tool and its input and offering permission options. The client replies
with the chosen option id. This is the single point Lazyboy intercepts:
the bridge writes the durable `approvals` row the moment this request
lands, and only sends the reply once a human resolves it in the
timeline.

A live model turn requires a configured provider (`goose configure`);
absent one, `initialize` still succeeds and the transport contract
above is fully exercisable. Bridge code is written against this
contract and tested with an in-process fake; the real binary is a
config-time swap.
