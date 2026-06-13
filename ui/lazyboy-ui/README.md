# lazyboy-ui

One React 19 + TypeScript app, Tailwind v4 + shadcn-style primitives, that
ships to both the Tauri desktop shell and the browser. The Claude-cowork-shaped
workspace: a space rail, a timeline where talk, agent messages, and tool
requests interleave, inline approval cards that gate every outside-world action,
and a task/run panel. See [`../../DOCS/SCOPE.md`](../../DOCS/SCOPE.md) →
"UI: one React app, two shells".

## The one rule

The app imports exactly one boundary, `RpcClient` (`src/rpc/client.ts`). It
never imports a transport directly — not `@tauri-apps/api`, not a raw `fetch`
to the core (SCOPE R2). The shell picks the implementation in
`src/shell/select.ts`, in this order:

- `TauriRpcClient` — desktop, core in-process over Tauri commands + events
  (chosen when a Tauri runtime is present).
- `HttpRpcClient` — browser, axum core over HTTP + SSE, CORS-aware (the
  default in a browser).
- `MockRpcClient` — in-memory cowork fixture, no backend, only when
  `VITE_USE_MOCK=1`.

## Develop

```bash
npm install
npm run build      # tsc -b && vite build
```

The browser shell talks to a real `lazyboy-server` by default. Point it at
one with env (see `.env.example`):

```bash
# against a running server (matches the server's LAZYBOY_ADDR / LAZYBOY_TOKEN)
VITE_CORE_URL=http://localhost:7878 VITE_CORE_TOKEN=devtoken npm run dev

# no backend, in-memory fixtures only
VITE_USE_MOCK=1 npm run dev
```

| Env var | Meaning | Default |
| --- | --- | --- |
| `VITE_CORE_URL` | Base URL of the `lazyboy-server` to talk to | `http://localhost:7878` |
| `VITE_CORE_TOKEN` | Single-tenant bearer; matches the server `LAZYBOY_TOKEN` | empty (auth disabled) |
| `VITE_USE_MOCK` | `1` forces the in-memory fixture | unset (real transport) |
