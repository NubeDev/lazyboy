import type { RpcClient } from "@/rpc/client";
import { MockRpcClient } from "@/rpc/mock";

// The shell — not the UI — decides which RpcClient implementation backs
// the app (SCOPE R2). Resolution order:
//   1. VITE_RPC=mock forces the in-memory fixture (default in dev).
//   2. A Tauri runtime present -> TauriRpcClient (desktop).
//   3. Otherwise the HTTP+SSE client against the axum core (browser).
// The real transports are loaded lazily so a mock-only dev build never
// pulls in the Tauri API or a live-server dependency.
export async function selectClient(): Promise<RpcClient> {
  const mode = import.meta.env.VITE_RPC ?? "mock";

  if (mode === "mock") return new MockRpcClient();

  if (mode === "tauri" || "__TAURI_INTERNALS__" in window) {
    const { TauriRpcClient } = await import("./tauri");
    return new TauriRpcClient();
  }

  const { HttpRpcClient } = await import("./http");
  const base = import.meta.env.VITE_CORE_URL ?? "http://localhost:7180";
  return new HttpRpcClient(base);
}
