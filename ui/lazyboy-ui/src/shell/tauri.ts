import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
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

// Desktop shell. Each method maps to a Tauri command that calls
// lazyboy-core in-process; the change stream is a Tauri event channel.
// This file is the ONLY place `@tauri-apps/api` is imported (SCOPE R2);
// it is lazily loaded so non-desktop builds never bundle it.
export class TauriRpcClient implements RpcClient {
  listSpaces() {
    return invoke<Space[]>("list_spaces");
  }
  timeline(spaceId: string) {
    return invoke<Message[]>("timeline", { spaceId });
  }
  listPending(spaceId: string) {
    return invoke<Approval[]>("list_pending", { spaceId });
  }
  listTasks(spaceId: string) {
    return invoke<Task[]>("list_tasks", { spaceId });
  }
  listRuns(spaceId: string) {
    return invoke<AgentRun[]>("list_runs", { spaceId });
  }
  startRun(spaceId: string, prompt: string) {
    return invoke<RunOutcome>("start_run", { spaceId, prompt });
  }
  decide(approvalId: string, status: ApprovalStatus) {
    return invoke<RunOutcome>("decide", { approvalId, status });
  }

  subscribe(spaceId: string, cb: () => void): () => void {
    const unlisten = listen(`space:${spaceId}`, () => cb());
    return () => void unlisten.then((fn) => fn());
  }
}
