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
    if (!res.ok) throw new Error(await this.errorText(`POST ${path}`, res));
    return res.json() as Promise<T>;
  }

  // The core returns `{error}` on a 4xx (a slug conflict, a malformed
  // body); surface that message so the banner explains the failure rather
  // than just the status line.
  private async errorText(label: string, res: Response): Promise<string> {
    const detail = await res
      .json()
      .then((b: { error?: string }) => b?.error)
      .catch(() => undefined);
    return detail ? `${label}: ${detail}` : `${label} -> ${res.status}`;
  }

  private headers(): Record<string, string> {
    return this.token ? { authorization: `Bearer ${this.token}` } : {};
  }

  private q(workspaceId: string): string {
    return `?workspace_id=${encodeURIComponent(workspaceId)}`;
  }

  health() {
    return this.get<Health>("/health");
  }
  listGooseProviders() {
    return this.get<GooseProvider[]>("/goose/providers");
  }
  getGooseConfig() {
    return this.get<GooseConfig>("/goose/config");
  }
  setGooseConfig(body: SetGooseConfigBody) {
    return this.post<GooseConfig>("/goose/config", body);
  }
  listSpaces() {
    return this.get<Space[]>("/spaces");
  }
  createSpace(slug: string, title: string) {
    return this.post<Space>("/spaces", { slug, title });
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
  createTask(spaceId: string, title: string) {
    return this.post<Task>(`/spaces/${spaceId}/tasks`, { title });
  }
  startRun(spaceId: string, prompt: string) {
    return this.post<RunOutcome>(`/spaces/${spaceId}/run`, { prompt });
  }
  decide(approvalId: string, status: ApprovalStatus) {
    return this.post<RunOutcome>(`/approvals/${approvalId}/decision`, { status });
  }

  listDecisions(spaceId: string) {
    return this.get<Decision[]>(`/spaces/${spaceId}/decisions`);
  }
  recordDecision(spaceId: string, body: RecordDecisionBody) {
    return this.post<Decision>(`/spaces/${spaceId}/decisions`, body);
  }
  listReminders(spaceId: string) {
    return this.get<Reminder[]>(`/spaces/${spaceId}/reminders`);
  }
  createReminder(spaceId: string, body: CreateReminderBody) {
    return this.post<Reminder>(`/spaces/${spaceId}/reminders`, body);
  }
  dismissReminder(reminderId: string) {
    return this.post<Reminder>(`/reminders/${reminderId}/dismiss`, {});
  }
  listCalendar(spaceId: string) {
    return this.get<CalendarEvent[]>(`/spaces/${spaceId}/calendar`);
  }
  upsertCalendar(spaceId: string, body: UpsertCalendarBody) {
    return this.post<CalendarEvent>(`/spaces/${spaceId}/calendar`, body);
  }

  listIntegrations(workspaceId: string) {
    return this.get<Integration[]>(`/integrations${this.q(workspaceId)}`);
  }
  createIntegration(body: CreateIntegrationBody) {
    return this.post<Integration>("/integrations", body);
  }
  ingress(integrationId: string, payload: unknown, spaceId?: string) {
    return this.post<IngestResult>(`/integrations/${integrationId}/ingress`, {
      payload,
      space_id: spaceId ?? null,
    });
  }
  setFeedVisibility(
    integrationId: string,
    spaceId: string,
    principalKind: string,
    principalId: string,
    mode: string,
  ) {
    return this.post<CreatedId>(`/feeds/${integrationId}/visibility`, {
      space_id: spaceId,
      principal_kind: principalKind,
      principal_id: principalId,
      mode,
    });
  }

  listWorkflows(workspaceId: string) {
    return this.get<Workflow[]>(`/workflows${this.q(workspaceId)}`);
  }
  createWorkflow(body: CreateWorkflowBody) {
    return this.post<Workflow>("/workflows", body);
  }
  enableWorkflow(workflowId: string) {
    return this.post<Workflow>(`/workflows/${workflowId}/enable`, {});
  }
  disableWorkflow(workflowId: string) {
    return this.post<Workflow>(`/workflows/${workflowId}/disable`, {});
  }
  fireWorkflow(workflowId: string, spaceId: string) {
    return this.post<RunOutcome>(`/workflows/${workflowId}/fire`, { space_id: spaceId });
  }

  listMembers(spaceId: string) {
    return this.get<Membership[]>(`/spaces/${spaceId}/members`);
  }
  createGroup(workspaceId: string, name: string) {
    return this.post<Group>("/groups", { workspace_id: workspaceId, name });
  }
  // The route returns 200/201 with no body, so this resolves to void
  // rather than parsing JSON.
  async addGroupMember(groupId: string, identityId: string) {
    const res = await fetch(`${this.base}/groups/${groupId}/members`, {
      method: "POST",
      headers: { ...this.headers(), "content-type": "application/json" },
      body: JSON.stringify({ identity_id: identityId }),
    });
    if (!res.ok) throw new Error(`POST /groups/${groupId}/members -> ${res.status}`);
  }
  grantMembership(spaceId: string, principalKind: string, principalId: string, role: string) {
    return this.post<CreatedId>(`/spaces/${spaceId}/members`, {
      principal_kind: principalKind,
      principal_id: principalId,
      role,
    });
  }

  subscribe(spaceId: string, cb: () => void): () => void {
    const url = new URL(`${this.base}/spaces/${spaceId}/subscribe`);
    if (this.token) url.searchParams.set("token", this.token);
    const es = new EventSource(url);
    es.onmessage = () => cb();
    return () => es.close();
  }
}
