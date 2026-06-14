import type {
  AgentRun,
  Approval,
  ApprovalStatus,
  CalendarEvent,
  CreateIntegrationBody,
  CreateReminderBody,
  CreateWorkflowBody,
  CreatedId,
  Decision,
  GooseConfig,
  GooseProvider,
  Group,
  Health,
  IngestResult,
  Integration,
  SetGooseConfigBody,
  Membership,
  Message,
  RecordDecisionBody,
  Reminder,
  RunOutcome,
  Space,
  Task,
  UpsertCalendarBody,
  Workflow,
} from "./types";

// The single boundary the React app is allowed to import (SCOPE R2).
// The shell injects an implementation: TauriRpcClient (desktop),
// HttpRpcClient (browser), or MockRpcClient (dev, no backend). The UI
// never knows which one it got. Every method maps one-to-one to a
// lazyboy-server route (and its Tauri command twin), so the surface here
// is the full server contract, not a subset.
export interface RpcClient {
  // Reachability of the goose backend; the status surface polls it.
  health(): Promise<Health>;

  // Goose provider management: list the providers, read the current
  // selection, and apply a new one (which relaunches goose).
  listGooseProviders(): Promise<GooseProvider[]>;
  getGooseConfig(): Promise<GooseConfig>;
  setGooseConfig(body: SetGooseConfigBody): Promise<GooseConfig>;

  listSpaces(): Promise<Space[]>;
  createSpace(slug: string, title: string): Promise<Space>;
  timeline(spaceId: string): Promise<Message[]>;
  listPending(spaceId: string): Promise<Approval[]>;
  listTasks(spaceId: string): Promise<Task[]>;
  listRuns(spaceId: string): Promise<AgentRun[]>;

  // Deterministic quick-add (the `/task` command): opens a task with no
  // agent run, so it costs no model turn. Natural-language task creation
  // goes through `startRun` and the agent's `create_task` tool instead.
  createTask(spaceId: string, title: string): Promise<Task>;
  startRun(spaceId: string, prompt: string): Promise<RunOutcome>;
  decide(approvalId: string, status: ApprovalStatus): Promise<RunOutcome>;

  // Durable-memory surfaces, all scoped to a space.
  listDecisions(spaceId: string): Promise<Decision[]>;
  recordDecision(spaceId: string, body: RecordDecisionBody): Promise<Decision>;
  listReminders(spaceId: string): Promise<Reminder[]>;
  createReminder(spaceId: string, body: CreateReminderBody): Promise<Reminder>;
  dismissReminder(reminderId: string): Promise<Reminder>;
  listCalendar(spaceId: string): Promise<CalendarEvent[]>;
  upsertCalendar(spaceId: string, body: UpsertCalendarBody): Promise<CalendarEvent>;

  // Workspace-scoped surfaces (integrations, workflows). The workspace id
  // comes off any Space.
  listIntegrations(workspaceId: string): Promise<Integration[]>;
  createIntegration(body: CreateIntegrationBody): Promise<Integration>;
  ingress(integrationId: string, payload: unknown, spaceId?: string): Promise<IngestResult>;
  setFeedVisibility(
    integrationId: string,
    spaceId: string,
    principalKind: string,
    principalId: string,
    mode: string,
  ): Promise<CreatedId>;

  listWorkflows(workspaceId: string): Promise<Workflow[]>;
  createWorkflow(body: CreateWorkflowBody): Promise<Workflow>;
  enableWorkflow(workflowId: string): Promise<Workflow>;
  disableWorkflow(workflowId: string): Promise<Workflow>;
  fireWorkflow(workflowId: string, spaceId: string): Promise<RunOutcome>;

  // Membership model (modeled, not enforced in the MVP gate, SCOPE R4).
  listMembers(spaceId: string): Promise<Membership[]>;
  createGroup(workspaceId: string, name: string): Promise<Group>;
  addGroupMember(groupId: string, identityId: string): Promise<void>;
  grantMembership(
    spaceId: string,
    principalKind: string,
    principalId: string,
    role: string,
  ): Promise<CreatedId>;

  // A change in the space (new message, approval, run state) fires `cb`.
  // SSE on the browser, a Tauri event on desktop, a timer on the mock.
  subscribe(spaceId: string, cb: () => void): () => void;
}
