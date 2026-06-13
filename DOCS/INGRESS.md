# Ingress (build step 3)

External events become timeline messages in a bound space, deduped
through `ingress_events` so a redelivery never doubles a message
(SCOPE.md "Integrations as ingress"). This note records the load-bearing
decisions and the explicit MVP boundaries.

## Pipeline

```
provider webhook/poll (serde_json::Value)
        |  lazyboy-ingress::normalize(provider, payload)
        v
NormalizedEvent { external_id, kind, body }
        |  resolve space: explicit body.space_id, else config_json bindings
        v
repo::ingress::ingest  -- dedup on UNIQUE(integration_id, external_id)
        |  miss: append `ingress` message, insert ingress_events row
        v
message in the space timeline
```

The HTTP sink is `POST /integrations/{id}/ingress` (lazyboy-server).
`GET/POST /integrations` register and list feeds; the create body
carries `secret_ref` only — a host secrets-store pointer, never a raw
credential (SCOPE.md R5).

## Dedup invariant

An `(integration_id, external_id)` pair maps to at most one timeline
message, forever. `ingest` checks `ingress_events` first and returns the
existing `message_id` on a hit (`deduped: true`) without appending a
second message. The `UNIQUE(integration_id, external_id)` constraint is
the backstop for a lost race between two concurrent deliveries.

## Routing model (MVP: explicit binding)

A space subscribes to a repo / label / thread / channel via bindings
stored in `integrations.config_json`:

```json
{ "bindings": [ { "repo": "owner/x", "space_id": "<uuid>" } ] }
```

`lazyboy-ingress::resolve_space` reads the provider-specific routing key
straight from the payload and returns the first binding whose declared
keys all match (a keyless binding is a catch-all). Content-based
auto-routing is out of MVP scope (SCOPE.md open question 3).

## Crate direction

`lazyboy-ingress` is pure: `Value` in, `NormalizedEvent` out, plus
binding resolution. It carries no process, socket, or HTTP dependency,
so it stays inside the mobile-safe crate graph (codeless R1). Live
provider API clients — GitHub/Gmail fetching, OAuth, webhook signature
verification — are **not** built here. **TODO (post-MVP):** add a
host-only fetch layer (in `lazyboy-adapters-host` or behind a cargo
feature), never inside `lazyboy-ingress`, so the normalization layer
never gains a transport dependency that would taint mobile-safe crates.

## Outbound

Outbound agent actions (open a PR, reply to a thread, create a calendar
event) are **not** a separate mechanism. They are Goose MCP extension
tool calls, and every outside-world tool call already parks an
`approvals` row under the existing approval flow (SCOPE.md R6). There is
nothing to build here for ingress step 3 beyond confirming that path is
the outbound path.
