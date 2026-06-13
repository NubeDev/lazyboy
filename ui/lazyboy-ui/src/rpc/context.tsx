import { createContext, useContext, type ReactNode } from "react";
import type { RpcClient } from "./client";

const RpcContext = createContext<RpcClient | null>(null);

// The shell wraps the app in this with its injected client; the UI only
// ever reaches the boundary through `useRpc()` (SCOPE R2).
export function RpcProvider({ client, children }: { client: RpcClient; children: ReactNode }) {
  return <RpcContext.Provider value={client}>{children}</RpcContext.Provider>;
}

export function useRpc(): RpcClient {
  const client = useContext(RpcContext);
  if (!client) throw new Error("useRpc used outside an RpcProvider");
  return client;
}
