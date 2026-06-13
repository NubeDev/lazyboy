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
`src/shell/select.ts`:

- `MockRpcClient` — in-memory cowork fixture, no backend (dev default).
- `HttpRpcClient` — browser, axum core over HTTP + SSE, CORS-aware.
- `TauriRpcClient` — desktop, core in-process over Tauri commands + events.

## Develop

```bash
npm install
npm run dev        # http://localhost:5180, mock data, no backend needed
npm run build      # tsc -b && vite build
```

Select a transport with env:

```bash
VITE_RPC=http VITE_CORE_URL=http://localhost:7180 npm run dev
```
