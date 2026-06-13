import type { RpcClient } from "@/rpc/client";
import type {
  AgentRun,
  Approval,
  ApprovalStatus,
  Message,
  RunOutcome,
  Space,
  Task,
} from "@/rpc/types";

// Browser shell. Talks to the axum core over HTTP for reads/commands and
// SSE for the per-space change stream. CORS is handled server-side; the
// single-tenant bearer (SCOPE R4) rides the Authorization header.
export class HttpRpcClient implements RpcClient {
  constructor(
    private base: string,
    private token = import.meta.env.VITE_CORE_TOKEN ?? "",
  ) {}

  private async get<T>(path: string): Promise<T> {
    const res = await fetch(`${this.base}${path}`, { headers: this.headers() });
    if (!res.ok) throw new Error(`GET ${path} -> ${res.status}`);
    return res.json() as Promise<T>;
  }

  private async post<T>(path: string, body: unknown): Promise<T> {
    const res = await fetch(`${this.base}${path}`, {
      method: "POST",
      headers: { ...this.headers(), "content-type": "application/json" },
      body: JSON.stringify(body),
    });
    if (!res.ok) throw new Error(`POST ${path} -> ${res.status}`);
    return res.json() as Promise<T>;
  }

  private headers(): Record<string, string> {
    return this.token ? { authorization: `Bearer ${this.token}` } : {};
  }

  listSpaces() {
    return this.get<Space[]>("/spaces");
  }
  timeline(spaceId: string) {
    return this.get<Message[]>(`/spaces/${spaceId}/timeline`);
  }
  listPending(spaceId: string) {
    return this.get<Approval[]>(`/spaces/${spaceId}/pending`);
  }
  listTasks(spaceId: string) {
    return this.get<Task[]>(`/spaces/${spaceId}/tasks`);
  }
  listRuns(spaceId: string) {
    return this.get<AgentRun[]>(`/spaces/${spaceId}/runs`);
  }
  startRun(spaceId: string, prompt: string) {
    return this.post<RunOutcome>(`/spaces/${spaceId}/run`, { prompt });
  }
  decide(approvalId: string, status: ApprovalStatus) {
    return this.post<RunOutcome>(`/approvals/${approvalId}/decision`, { status });
  }

  subscribe(spaceId: string, cb: () => void): () => void {
    const url = new URL(`${this.base}/spaces/${spaceId}/subscribe`);
    if (this.token) url.searchParams.set("token", this.token);
    const es = new EventSource(url);
    es.onmessage = () => cb();
    return () => es.close();
  }
}
