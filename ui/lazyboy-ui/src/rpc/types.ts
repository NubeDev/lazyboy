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
  // null for a chat turn (a run with no task); set for a task-backed run.
  task_id: string | null;
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

export type ReminderStatus = "pending" | "fired" | "dismissed";
export type Provider = "github" | "gmail" | "slack" | "gcal";
export type TriggerKind = "feed" | "schedule";
export type ApprovalPolicy = "require_approval" | "auto_approve";
export type WorkflowStatus = "enabled" | "disabled";

// Mirrors DecisionDto: a durable-memory record of a resolved question,
// optionally anchored to the timeline message that prompted it.
export interface Decision {
  id: string;
  space_id: string;
  message_id: string | null;
  summary: string;
  decided_by_identity_id: string | null;
  decided_at: string;
}

// Mirrors ReminderDto. `due_at`/`status` drive the panel's overdue and
// dismissed states.
export interface Reminder {
  id: string;
  space_id: string;
  task_id: string | null;
  due_at: string;
  body: string;
  status: ReminderStatus;
}

// Mirrors CalendarEventDto. `external_ref` present marks a synced event
// (the upsert dedups on it); absent is a local event.
export interface CalendarEvent {
  id: string;
  space_id: string;
  source: string;
  external_ref: string | null;
  title: string;
  starts_at: string;
  ends_at: string | null;
  meta_json: string | null;
}

// Mirrors IntegrationDto. `secret_ref` names a host secrets-store entry,
// never a raw token (SCOPE R5), so it is safe to render.
export interface Integration {
  id: string;
  workspace_id: string;
  provider: Provider;
  account_ref: string | null;
  secret_ref: string | null;
  status: string;
  config_json: string | null;
}

// Mirrors WorkflowDto. `status === "enabled"` is an armed automation;
// `approval_policy` is the per-workflow gate.
export interface Workflow {
  id: string;
  workspace_id: string;
  name: string;
  trigger_kind: TriggerKind;
  trigger_config_json: string | null;
  approval_policy: ApprovalPolicy;
  steps_json: string;
  status: WorkflowStatus;
}

export interface Group {
  id: string;
  workspace_id: string;
  name: string;
}

// Mirrors MembershipDto: a user or group granted a role in a space.
// Modeled, not enforced in the MVP trust gate (SCOPE R4).
export interface Membership {
  id: string;
  space_id: string;
  principal_kind: string;
  principal_id: string;
  role: string;
}

// Body shapes for the create/record commands. These mirror the
// `Deserialize` bodies in lazyboy-wire; timestamps cross as RFC3339.
export interface RecordDecisionBody {
  summary: string;
  message_id?: string | null;
  decided_by_identity_id?: string | null;
}

export interface CreateReminderBody {
  body: string;
  due_at: string;
  task_id?: string | null;
}

export interface UpsertCalendarBody {
  source: string;
  title: string;
  starts_at: string;
  external_ref?: string | null;
  ends_at?: string | null;
  meta_json?: string | null;
}

export interface CreateIntegrationBody {
  workspace_id: string;
  provider: Provider;
  account_ref?: string | null;
  secret_ref?: string | null;
  config_json?: unknown;
}

export interface CreateWorkflowBody {
  workspace_id: string;
  name: string;
  trigger_kind: TriggerKind;
  trigger_config_json?: string | null;
  approval_policy: ApprovalPolicy;
  steps_json: string;
}

// Mirrors HealthDto: reachability of the goose backend the node drives
// work through. `goose_detail` carries the failure reason when goose is
// unreachable, so the UI can show why, not just that it is down.
export interface Health {
  goose_url: string;
  goose_reachable: boolean;
  goose_detail: string | null;
}

// Mirrors GooseProviderDto: one selectable goose provider. `key_set`
// reflects whether a key is already stored (never the key itself, SCOPE
// R5); `requires_key` drives whether the form demands one.
export interface GooseProvider {
  id: string;
  display_name: string;
  requires_key: boolean;
  key_set: boolean;
  models: string[];
}

// Mirrors GooseConfigDto: the current provider selection plus whether
// goose is running under it.
export interface GooseConfig {
  provider: string | null;
  model: string | null;
  running: boolean;
}

// Mirrors SetGooseConfigBody. `api_key` omitted keeps the stored key; an
// empty string clears it.
export interface SetGooseConfigBody {
  provider: string;
  model?: string | null;
  api_key?: string | null;
}

// Mirrors IngestResultDto: the timeline message the event mapped to and
// whether the POST was a deduped redelivery.
export interface IngestResult {
  message_id: string;
  deduped: boolean;
}

// Mirrors CreatedIdDto: the id of a created membership/visibility row.
export interface CreatedId {
  id: string;
}
