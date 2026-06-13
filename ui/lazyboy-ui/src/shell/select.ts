import type { RpcClient } from "@/rpc/client";

// The shell — not the UI — decides which RpcClient implementation backs
// the app (SCOPE R2). Resolution order:
//   1. A Tauri runtime present -> TauriRpcClient (desktop).
//   2. Otherwise the HTTP+SSE client against the axum core (browser).
//   3. VITE_USE_MOCK=1 forces the in-memory fixture, for UI dev with no
//      backend. It is never the default — the real transports are.
// Implementations load lazily so a browser build never pulls in the Tauri
// API and a mock-only dev build never pulls in either real transport.
export async function selectClient(): Promise<RpcClient> {
  if (import.meta.env.VITE_USE_MOCK === "1") {
    const { MockRpcClient } = await import("@/rpc/mock");
    return new MockRpcClient();
  }

  if ("__TAURI_INTERNALS__" in window) {
    const { TauriRpcClient } = await import("./tauri");
    return new TauriRpcClient();
  }

  const { HttpRpcClient } = await import("./http");
  const base = import.meta.env.VITE_CORE_URL ?? "http://localhost:7878";
  return new HttpRpcClient(base);
}
