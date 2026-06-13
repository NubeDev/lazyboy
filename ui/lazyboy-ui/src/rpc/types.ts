// Wire types. Each interface mirrors a `lazyboy-wire` DTO field-for-field,
// including snake_case names, so the same JSON crosses the HTTP and Tauri
// transports unchanged. Keep these identical to the Rust side
// (crates/lazyboy-wire/src/lib.rs); a divergence is a wire bug, not a UI
// choice. String unions are the snake_case serde forms of the domain enums
// in lazyboy-types.

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
  workspace_id: string;
  slug: string;
  title: string;
  status: string;
}

// MessageDto carries no author column; the timeline derives a display
// author from `kind` (see lib/labels). `ref_id` links a tool_request to
// its pending approval.
export interface Message {
  id: string;
  space_id: string;
  kind: MessageKind;
  body: string;
  ts: string;
  ref_id: string | null;
}

export interface Task {
  id: string;
  space_id: string;
  title: string;
  state: TaskState;
  agent_run_id: string | null;
}

// RunDto has no timestamps: the store tracks status, not start/end times.
export interface AgentRun {
  id: string;
  space_id: string;
  task_id: string;
  goose_session_id: string | null;
  status: RunStatus;
}

export interface Approval {
  id: string;
  space_id: string;
  agent_run_id: string;
  goose_session_id: string;
  tool_name: string;
  tool_input_json: string;
  status: ApprovalStatus;
}

// Mirrors RunOutcomeDto: a `#[serde(tag = "outcome")]` enum. `decide`
// returns `already_resolved` when a racing client resolved the approval
// first (two browser tabs, one tenant).
export type RunOutcome =
  | { outcome: "awaiting_approval" }
  | { outcome: "ended"; succeeded: boolean }
  | { outcome: "already_resolved" };
