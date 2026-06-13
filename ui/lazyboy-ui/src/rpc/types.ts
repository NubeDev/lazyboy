// Wire types. Each string union is the snake_case serde form already
// emitted by the Rust domain enums (lazyboy-types), so the same JSON
// crosses the Tauri and HTTP transports unchanged. Keep these identical
// to the Rust side; a divergence is a wire bug, not a UI choice.

export type MessageKind =
  | "human"
  | "agent"
  | "system"
  | "tool_request"
  | "tool_result"
  | "artifact_ref"
  | "decision_ref"
  | "ingress";

export type RunStatus =
  | "queued"
  | "running"
  | "waiting_approval"
  | "succeeded"
  | "failed"
  | "cancelled";

export type TaskState =
  | "open"
  | "running"
  | "blocked_on_approval"
  | "done"
  | "cancelled";

export type ApprovalStatus = "pending" | "approved" | "denied";

export interface Space {
  id: string;
  slug: string;
  title: string;
  status: "active" | "archived";
}

export interface Message {
  id: string;
  spaceId: string;
  authorName: string;
  kind: MessageKind;
  body: string;
  ts: string;
  refId: string | null;
}

export interface Task {
  id: string;
  spaceId: string;
  title: string;
  state: TaskState;
}

export interface AgentRun {
  id: string;
  spaceId: string;
  taskId: string | null;
  status: RunStatus;
  startedAt: string;
  endedAt: string | null;
}

export interface Approval {
  id: string;
  spaceId: string;
  agentRunId: string;
  toolName: string;
  toolInputJson: string;
  status: ApprovalStatus;
  requestedAt: string;
}

// Mirrors lazyboy_core::RunOutcome.
export type RunOutcome =
  | { kind: "awaiting_approval" }
  | { kind: "ended"; succeeded: boolean };
