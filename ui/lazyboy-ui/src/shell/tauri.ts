import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { RpcClient } from "@/rpc/client";
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
  Membership,
  SetGooseConfigBody,
  Message,
  RecordDecisionBody,
  Reminder,
  RunOutcome,
  Space,
  Task,
  UpsertCalendarBody,
  Workflow,
} from "@/rpc/types";

// Desktop shell. Each method maps to a Tauri command that calls
// lazyboy-core in-process; the change stream is a Tauri event channel.
// This file is the ONLY place `@tauri-apps/api` is imported (SCOPE R2);
// it is lazily loaded so non-desktop builds never bundle it. Command
// names and argument keys match the wrappers in
// crates/lazyboy-tauri/src/app.rs (camelCase keys, Tauri converts to the
// command's snake_case params).
export class TauriRpcClient implements RpcClient {
  health() {
    return invoke<Health>("health");
  }
  listGooseProviders() {
    return invoke<GooseProvider[]>("list_goose_providers");
  }
  getGooseConfig() {
    return invoke<GooseConfig>("get_goose_config");
  }
  setGooseConfig(body: SetGooseConfigBody) {
    return invoke<GooseConfig>("set_goose_config", {
      provider: body.provider,
      model: body.model ?? null,
      apiKey: body.api_key ?? null,
    });
  }
  listSpaces() {
    return invoke<Space[]>("list_spaces");
  }
  createSpace(slug: string, title: string) {
    return invoke<Space>("create_space", { body: { slug, title } });
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
  createTask(spaceId: string, title: string) {
    return invoke<Task>("create_task", { spaceId, body: { title } });
  }
  startRun(spaceId: string, prompt: string) {
    return invoke<RunOutcome>("start_run", { spaceId, prompt });
  }
  decide(approvalId: string, status: ApprovalStatus) {
    return invoke<RunOutcome>("decide", { approvalId, status });
  }

  listDecisions(spaceId: string) {
    return invoke<Decision[]>("list_decisions", { spaceId });
  }
  recordDecision(spaceId: string, body: RecordDecisionBody) {
    return invoke<Decision>("record_decision", { spaceId, body });
  }
  listReminders(spaceId: string) {
    return invoke<Reminder[]>("list_reminders", { spaceId });
  }
  createReminder(spaceId: string, body: CreateReminderBody) {
    return invoke<Reminder>("create_reminder", { spaceId, body });
  }
  dismissReminder(reminderId: string) {
    return invoke<Reminder>("dismiss_reminder", { reminderId });
  }
  listCalendar(spaceId: string) {
    return invoke<CalendarEvent[]>("list_calendar", { spaceId });
  }
  upsertCalendar(spaceId: string, body: UpsertCalendarBody) {
    return invoke<CalendarEvent>("upsert_calendar", { spaceId, body });
  }

  listIntegrations(workspaceId: string) {
    return invoke<Integration[]>("list_integrations", { workspaceId });
  }
  createIntegration(body: CreateIntegrationBody) {
    return invoke<Integration>("create_integration", { body });
  }
  ingress(integrationId: string, payload: unknown, spaceId?: string) {
    return invoke<IngestResult>("ingress", {
      integrationId,
      payload,
      spaceId: spaceId ?? null,
    });
  }
  setFeedVisibility(
    integrationId: string,
    spaceId: string,
    principalKind: string,
    principalId: string,
    mode: string,
  ) {
    return invoke<CreatedId>("set_feed_visibility", {
      integrationId,
      spaceId,
      principalKind,
      principalId,
      mode,
    });
  }

  listWorkflows(workspaceId: string) {
    return invoke<Workflow[]>("list_workflows", { workspaceId });
  }
  createWorkflow(body: CreateWorkflowBody) {
    return invoke<Workflow>("create_workflow", { body });
  }
  enableWorkflow(workflowId: string) {
    return invoke<Workflow>("enable_workflow", { workflowId });
  }
  disableWorkflow(workflowId: string) {
    return invoke<Workflow>("disable_workflow", { workflowId });
  }
  fireWorkflow(workflowId: string, spaceId: string) {
    return invoke<RunOutcome>("fire_workflow", { workflowId, spaceId });
  }

  listMembers(spaceId: string) {
    return invoke<Membership[]>("list_members", { spaceId });
  }
  createGroup(workspaceId: string, name: string) {
    return invoke<Group>("create_group", { workspaceId, name });
  }
  async addGroupMember(groupId: string, identityId: string) {
    await invoke("add_group_member", { groupId, identityId });
  }
  grantMembership(spaceId: string, principalKind: string, principalId: string, role: string) {
    return invoke<CreatedId>("grant_membership", {
      spaceId,
      principalKind,
      principalId,
      role,
    });
  }

  subscribe(spaceId: string, cb: () => void): () => void {
    const unlisten = listen(`space:${spaceId}`, () => cb());
    return () => void unlisten.then((fn) => fn());
  }
}
