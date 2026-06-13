import type {
  AgentRun,
  Approval,
  ApprovalStatus,
  Message,
  RunOutcome,
  Space,
  Task,
} from "./types";

// The single boundary the React app is allowed to import (SCOPE R2).
// The shell injects an implementation: TauriRpcClient (desktop),
// HttpRpcClient (browser), or MockRpcClient (dev, no backend). The UI
// never knows which one it got.
export interface RpcClient {
  listSpaces(): Promise<Space[]>;
  timeline(spaceId: string): Promise<Message[]>;
  listPending(spaceId: string): Promise<Approval[]>;
  listTasks(spaceId: string): Promise<Task[]>;
  listRuns(spaceId: string): Promise<AgentRun[]>;

  startRun(spaceId: string, prompt: string): Promise<RunOutcome>;
  decide(approvalId: string, status: ApprovalStatus): Promise<RunOutcome>;

  // A change in the space (new message, approval, run state) fires `cb`.
  // SSE on the browser, a Tauri event on desktop, a timer on the mock.
  subscribe(spaceId: string, cb: () => void): () => void;
}
