import type { RpcClient } from "./client";
import type {
  AgentRun,
  Approval,
  ApprovalStatus,
  Message,
  RunOutcome,
  Space,
  Task,
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

  private subs: Record<string, Set<() => void>> = {};

  async listSpaces() {
    return this.spaces;
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
