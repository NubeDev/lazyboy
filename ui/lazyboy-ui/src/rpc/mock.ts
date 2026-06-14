import type { RpcClient } from "./client";
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
} from "./types";

// In-memory cowork fixture so the full shell renders with no backend.
// Available only behind VITE_USE_MOCK=1 (see shell/select). Field shapes
// mirror lazyboy-wire exactly so swapping in a real transport is a no-op
// for the UI. Mutations (startRun, decide) update the in-memory timeline
// and notify subscribers, so the mock behaves like the real thing in dev.
let seq = 0;
const id = (p: string) => `${p}_${(seq++).toString(36)}`;

const WS = "ws_demo";

export class MockRpcClient implements RpcClient {
  private spaces: Space[] = [
    { id: "sp_pricing", workspace_id: WS, slug: "new-pricing-page", title: "New pricing page", status: "active" },
    { id: "sp_migration", workspace_id: WS, slug: "q3-migration", title: "Q3 migration", status: "active" },
    { id: "sp_acme", workspace_id: WS, slug: "acme-onboarding", title: "Acme onboarding", status: "active" },
  ];

  private messages: Record<string, Message[]> = {
    sp_pricing: [
      msg("sp_pricing", "human", "Let's ship the new pricing page. Start by drafting the three-tier table from the spec in docs/pricing.md.", "-2h"),
      msg("sp_pricing", "agent", "Reading docs/pricing.md and the existing PricingTable component. I'll draft a three-tier layout and propose the copy.", "-118m"),
      msg("sp_pricing", "tool_request", "read_file docs/pricing.md", "-117m", "ap_done"),
      msg("sp_pricing", "tool_result", "Read 84 lines. Tiers: Starter, Team, Enterprise.", "-116m"),
      msg("sp_pricing", "agent", "Drafted the table. To apply it I need to write src/components/PricingTable.tsx — that's a repo change, so it needs your approval.", "-3m"),
      msg("sp_pricing", "tool_request", "write_file src/components/PricingTable.tsx", "-2m", "ap_pending"),
    ],
    sp_migration: [
      msg("sp_migration", "human", "Track the Postgres 16 upgrade here. First audit which queries use the deprecated syntax.", "-1d"),
      msg("sp_migration", "agent", "On it. I'll grep the repo for the deprecated patterns and summarize by service.", "-1d"),
    ],
    sp_acme: [
      msg("sp_acme", "ingress", "GitHub: Acme opened issue #412 \"SSO callback 500s on staging\".", "-5h"),
    ],
  };

  private approvals: Record<string, Approval[]> = {
    sp_pricing: [
      {
        id: "ap_pending",
        space_id: "sp_pricing",
        agent_run_id: "run_pricing",
        goose_session_id: "gs_pricing",
        tool_name: "write_file",
        tool_input_json: JSON.stringify(
          { path: "src/components/PricingTable.tsx", bytes: 2841 },
          null,
          2,
        ),
        status: "pending",
      },
    ],
  };

  private tasks: Record<string, Task[]> = {
    sp_pricing: [
      { id: "tk_table", space_id: "sp_pricing", title: "Draft three-tier pricing table", state: "blocked_on_approval", agent_run_id: "run_pricing" },
      { id: "tk_copy", space_id: "sp_pricing", title: "Write tier copy", state: "open", agent_run_id: null },
    ],
    sp_migration: [
      { id: "tk_audit", space_id: "sp_migration", title: "Audit deprecated query syntax", state: "running", agent_run_id: "run_mig" },
    ],
    sp_acme: [],
  };

  private runs: Record<string, AgentRun[]> = {
    sp_pricing: [
      { id: "run_pricing", space_id: "sp_pricing", task_id: "tk_table", goose_session_id: "gs_pricing", status: "waiting_approval" },
    ],
    sp_migration: [
      { id: "run_mig", space_id: "sp_migration", task_id: "tk_audit", goose_session_id: "gs_mig", status: "running" },
    ],
    sp_acme: [],
  };

  private decisions: Record<string, Decision[]> = {
    sp_pricing: [
      {
        id: "dec_tiers",
        space_id: "sp_pricing",
        message_id: null,
        summary: "Ship three tiers: Starter, Team, Enterprise. Drop the free tier for now.",
        decided_by_identity_id: "id_human",
        decided_at: rel("-90m"),
      },
    ],
  };

  private reminders: Record<string, Reminder[]> = {
    sp_pricing: [
      {
        id: "rem_copy",
        space_id: "sp_pricing",
        task_id: "tk_copy",
        due_at: rel("-30m"),
        body: "Review the Enterprise tier copy before the marketing sync.",
        status: "pending",
      },
    ],
    sp_migration: [
      {
        id: "rem_cutover",
        space_id: "sp_migration",
        task_id: null,
        due_at: relFuture("2d"),
        body: "Postgres 16 cutover window opens.",
        status: "pending",
      },
    ],
  };

  private calendar: Record<string, CalendarEvent[]> = {
    sp_pricing: [
      {
        id: "cal_sync",
        space_id: "sp_pricing",
        source: "gcal",
        external_ref: "evt_123",
        title: "Marketing sync",
        starts_at: relFuture("3h"),
        ends_at: relFuture("4h"),
        meta_json: null,
      },
    ],
  };

  private integrations: Integration[] = [
    {
      id: "int_github",
      workspace_id: WS,
      provider: "github",
      account_ref: "nube-io/lazyboy",
      secret_ref: "secret://github-pat",
      status: "connected",
      config_json: JSON.stringify({ bindings: { "issues:nube-io/lazyboy": "sp_acme" } }),
    },
  ];

  private workflows: Workflow[] = [
    {
      id: "wf_triage",
      workspace_id: WS,
      name: "Triage new GitHub issues",
      trigger_kind: "feed",
      trigger_config_json: JSON.stringify({ provider: "github", event: "issues.opened" }),
      approval_policy: "require_approval",
      steps_json: JSON.stringify([
        { prompt: "Read the issue, label it, and propose a first response." },
      ]),
      status: "enabled",
    },
  ];

  private groups: Group[] = [];

  private members: Record<string, Membership[]> = {
    sp_pricing: [
      {
        id: "mem_seed",
        space_id: "sp_pricing",
        principal_kind: "user",
        principal_id: "id_human",
        role: "owner",
      },
    ],
  };

  private gooseProviders: GooseProvider[] = [
    {
      id: "anthropic",
      display_name: "Anthropic (Claude)",
      requires_key: true,
      key_set: true,
      models: ["claude-opus-4-20250514", "claude-sonnet-4-20250514"],
    },
    {
      id: "openai",
      display_name: "OpenAI",
      requires_key: true,
      key_set: false,
      models: ["gpt-4o", "gpt-4o-mini"],
    },
    {
      id: "ollama",
      display_name: "Ollama (local)",
      requires_key: false,
      key_set: false,
      models: ["llama3.3", "qwen2.5"],
    },
  ];

  private gooseConfig: GooseConfig = {
    provider: "anthropic",
    model: "claude-sonnet-4-20250514",
    running: true,
  };

  private subs: Record<string, Set<() => void>> = {};

  async health(): Promise<Health> {
    return {
      goose_url: "mock://goose",
      goose_reachable: this.gooseConfig.running,
      goose_detail: null,
    };
  }
  async listGooseProviders() {
    return this.gooseProviders;
  }
  async getGooseConfig() {
    return this.gooseConfig;
  }
  async setGooseConfig(body: SetGooseConfigBody): Promise<GooseConfig> {
    const provider = this.gooseProviders.find((p) => p.id === body.provider);
    if (!provider) throw new Error(`unknown provider: ${body.provider}`);
    if (body.api_key !== undefined && body.api_key !== null) {
      provider.key_set = body.api_key !== "";
    }
    this.gooseConfig = {
      provider: body.provider,
      model: body.model ?? null,
      running: !provider.requires_key || provider.key_set,
    };
    return this.gooseConfig;
  }
  async listSpaces() {
    return this.spaces;
  }
  async createSpace(slug: string, title: string): Promise<Space> {
    const key = slug.trim();
    if (!key) throw new Error("slug must not be empty");
    if (this.spaces.some((s) => s.slug === key)) throw new Error(`slug '${key}' already in use`);
    const space: Space = {
      id: `sp_${key}`,
      workspace_id: WS,
      slug: key,
      title: title.trim() || key,
      status: "active",
    };
    this.spaces = [...this.spaces, space];
    return space;
  }
  async timeline(spaceId: string) {
    return this.messages[spaceId] ?? [];
  }
  async listPending(spaceId: string) {
    return (this.approvals[spaceId] ?? []).filter((a) => a.status === "pending");
  }
  async listTasks(spaceId: string) {
    return this.tasks[spaceId] ?? [];
  }
  async listRuns(spaceId: string) {
    return this.runs[spaceId] ?? [];
  }

  async createTask(spaceId: string, title: string): Promise<Task> {
    const t: Task = {
      id: `tk_${Math.random().toString(36).slice(2, 8)}`,
      space_id: spaceId,
      title: title.trim(),
      state: "open",
      agent_run_id: null,
    };
    this.tasks[spaceId] = [...(this.tasks[spaceId] ?? []), t];
    this.notify(spaceId);
    return t;
  }

  async startRun(spaceId: string, prompt: string): Promise<RunOutcome> {
    this.push(spaceId, msg(spaceId, "human", prompt, "now"));
    this.push(spaceId, msg(spaceId, "agent", "Picking this up — I'll work it in this space and pause for approval before any outside change.", "now"));
    this.notify(spaceId);
    return { outcome: "ended", succeeded: true };
  }

  async decide(approvalId: string, status: ApprovalStatus): Promise<RunOutcome> {
    for (const spaceId of Object.keys(this.approvals)) {
      const ap = this.approvals[spaceId].find((a) => a.id === approvalId);
      if (!ap) continue;
      if (ap.status !== "pending") return { outcome: "already_resolved" };
      ap.status = status;
      const run = this.runs[spaceId]?.find((r) => r.id === ap.agent_run_id);
      if (status === "approved") {
        this.push(spaceId, msg(spaceId, "tool_result", `Applied ${ap.tool_name}. Wrote ${JSON.parse(ap.tool_input_json).path}.`, "now"));
        this.push(spaceId, msg(spaceId, "agent", "Done. The pricing table component is in place — want me to wire it into the route next?", "now"));
        if (run) run.status = "succeeded";
        this.setTaskState(spaceId, run?.task_id, "done");
      } else {
        this.push(spaceId, msg(spaceId, "system", `Denied ${ap.tool_name}. Holding — tell me what to do instead.`, "now"));
        if (run) run.status = "cancelled";
        this.setTaskState(spaceId, run?.task_id, "open");
      }
      this.notify(spaceId);
      return { outcome: "ended", succeeded: status === "approved" };
    }
    return { outcome: "already_resolved" };
  }

  async listDecisions(spaceId: string) {
    return this.decisions[spaceId] ?? [];
  }
  async recordDecision(spaceId: string, body: RecordDecisionBody): Promise<Decision> {
    const d: Decision = {
      id: id("dec"),
      space_id: spaceId,
      message_id: body.message_id ?? null,
      summary: body.summary,
      decided_by_identity_id: body.decided_by_identity_id ?? "id_human",
      decided_at: new Date(BASE).toISOString(),
    };
    (this.decisions[spaceId] ??= []).push(d);
    this.push(spaceId, msg(spaceId, "decision_ref", `Decision recorded: ${body.summary}`, "now"));
    this.notify(spaceId);
    return d;
  }

  async listReminders(spaceId: string) {
    return this.reminders[spaceId] ?? [];
  }
  async createReminder(spaceId: string, body: CreateReminderBody): Promise<Reminder> {
    const r: Reminder = {
      id: id("rem"),
      space_id: spaceId,
      task_id: body.task_id ?? null,
      due_at: body.due_at,
      body: body.body,
      status: "pending",
    };
    (this.reminders[spaceId] ??= []).push(r);
    this.notify(spaceId);
    return r;
  }
  async dismissReminder(reminderId: string): Promise<Reminder> {
    for (const spaceId of Object.keys(this.reminders)) {
      const r = this.reminders[spaceId].find((x) => x.id === reminderId);
      if (r) {
        r.status = "dismissed";
        this.notify(spaceId);
        return r;
      }
    }
    throw new Error(`reminder ${reminderId} not found`);
  }

  async listCalendar(spaceId: string) {
    return this.calendar[spaceId] ?? [];
  }
  async upsertCalendar(spaceId: string, body: UpsertCalendarBody): Promise<CalendarEvent> {
    const list = (this.calendar[spaceId] ??= []);
    const existing = body.external_ref
      ? list.find((e) => e.external_ref === body.external_ref)
      : undefined;
    const event: CalendarEvent = existing ?? {
      id: id("cal"),
      space_id: spaceId,
      source: body.source,
      external_ref: body.external_ref ?? null,
      title: body.title,
      starts_at: body.starts_at,
      ends_at: body.ends_at ?? null,
      meta_json: body.meta_json ?? null,
    };
    if (existing) {
      existing.title = body.title;
      existing.starts_at = body.starts_at;
      existing.ends_at = body.ends_at ?? null;
      existing.meta_json = body.meta_json ?? null;
    } else {
      list.push(event);
    }
    this.notify(spaceId);
    return event;
  }

  async listIntegrations(workspaceId: string) {
    return this.integrations.filter((i) => i.workspace_id === workspaceId);
  }
  async createIntegration(body: CreateIntegrationBody): Promise<Integration> {
    const i: Integration = {
      id: id("int"),
      workspace_id: body.workspace_id,
      provider: body.provider,
      account_ref: body.account_ref ?? null,
      secret_ref: body.secret_ref ?? null,
      status: "connected",
      config_json: body.config_json ? JSON.stringify(body.config_json) : null,
    };
    this.integrations.push(i);
    return i;
  }
  async ingress(integrationId: string, payload: unknown, spaceId?: string): Promise<IngestResult> {
    const target = spaceId ?? "sp_acme";
    const m = msg(target, "ingress", `Ingress via ${integrationId}: ${JSON.stringify(payload)}`, "now");
    this.push(target, m);
    this.notify(target);
    return { message_id: m.id, deduped: false };
  }
  async setFeedVisibility(): Promise<CreatedId> {
    return { id: id("vis") };
  }

  async listWorkflows(workspaceId: string) {
    return this.workflows.filter((w) => w.workspace_id === workspaceId);
  }
  async createWorkflow(body: CreateWorkflowBody): Promise<Workflow> {
    const w: Workflow = {
      id: id("wf"),
      workspace_id: body.workspace_id,
      name: body.name,
      trigger_kind: body.trigger_kind,
      trigger_config_json: body.trigger_config_json ?? null,
      approval_policy: body.approval_policy,
      steps_json: body.steps_json,
      status: "disabled",
    };
    this.workflows.push(w);
    return w;
  }
  async enableWorkflow(workflowId: string): Promise<Workflow> {
    return this.setWorkflowStatus(workflowId, "enabled");
  }
  async disableWorkflow(workflowId: string): Promise<Workflow> {
    return this.setWorkflowStatus(workflowId, "disabled");
  }
  private setWorkflowStatus(workflowId: string, status: Workflow["status"]): Workflow {
    const w = this.workflows.find((x) => x.id === workflowId);
    if (!w) throw new Error(`workflow ${workflowId} not found`);
    w.status = status;
    return w;
  }
  async fireWorkflow(workflowId: string, spaceId: string): Promise<RunOutcome> {
    const w = this.workflows.find((x) => x.id === workflowId);
    this.push(spaceId, msg(spaceId, "system", `Fired workflow "${w?.name ?? workflowId}".`, "now"));
    this.notify(spaceId);
    return { outcome: "ended", succeeded: true };
  }

  async listMembers(spaceId: string) {
    return this.members[spaceId] ?? [];
  }
  async createGroup(workspaceId: string, name: string): Promise<Group> {
    const g: Group = { id: id("grp"), workspace_id: workspaceId, name };
    this.groups.push(g);
    return g;
  }
  async addGroupMember(): Promise<void> {}
  async grantMembership(
    spaceId: string,
    principalKind: string,
    principalId: string,
    role: string,
  ): Promise<CreatedId> {
    const m: Membership = {
      id: id("mem"),
      space_id: spaceId,
      principal_kind: principalKind,
      principal_id: principalId,
      role,
    };
    (this.members[spaceId] ??= []).push(m);
    this.notify(spaceId);
    return { id: m.id };
  }

  subscribe(spaceId: string, cb: () => void) {
    (this.subs[spaceId] ??= new Set()).add(cb);
    return () => this.subs[spaceId]?.delete(cb);
  }

  private push(spaceId: string, m: Message) {
    (this.messages[spaceId] ??= []).push(m);
  }
  private setTaskState(spaceId: string, taskId: string | null | undefined, state: Task["state"]) {
    if (!taskId) return;
    const t = this.tasks[spaceId]?.find((t) => t.id === taskId);
    if (t) t.state = state;
  }
  private notify(spaceId: string) {
    this.subs[spaceId]?.forEach((cb) => cb());
  }
}

function msg(
  spaceId: string,
  kind: Message["kind"],
  body: string,
  when: string,
  refId: string | null = null,
): Message {
  return { id: id("m"), space_id: spaceId, kind, body, ts: rel(when), ref_id: refId };
}

// Fixed offsets from a stable base so the mock is deterministic across
// reloads (no Date.now churn in fixtures).
const BASE = Date.parse("2026-06-13T14:00:00Z");
function rel(when: string): string {
  if (when === "now") return new Date(BASE).toISOString();
  const m = /^-(\d+)([mhd])$/.exec(when);
  if (!m) return new Date(BASE).toISOString();
  const n = Number(m[1]);
  const mult = m[2] === "m" ? 60_000 : m[2] === "h" ? 3_600_000 : 86_400_000;
  return new Date(BASE - n * mult).toISOString();
}

function relFuture(when: string): string {
  const m = /^(\d+)([mhd])$/.exec(when);
  if (!m) return new Date(BASE).toISOString();
  const n = Number(m[1]);
  const mult = m[2] === "m" ? 60_000 : m[2] === "h" ? 3_600_000 : 86_400_000;
  return new Date(BASE + n * mult).toISOString();
}
