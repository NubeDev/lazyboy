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
// Mutations (startRun, decide) update the in-memory timeline and notify
// subscribers, so the UI behaves like the real thing during dev.
let seq = 0;
const id = (p: string) => `${p}_${(seq++).toString(36)}`;

export class MockRpcClient implements RpcClient {
  private spaces: Space[] = [
    { id: "sp_pricing", slug: "new-pricing-page", title: "New pricing page", status: "active" },
    { id: "sp_migration", slug: "q3-migration", title: "Q3 migration", status: "active" },
    { id: "sp_acme", slug: "acme-onboarding", title: "Acme onboarding", status: "active" },
  ];

  private messages: Record<string, Message[]> = {
    sp_pricing: [
      msg("sp_pricing", "Priya", "human", "Let's ship the new pricing page. Start by drafting the three-tier table from the spec in docs/pricing.md.", "-2h"),
      msg("sp_pricing", "Goose", "agent", "Reading docs/pricing.md and the existing PricingTable component. I'll draft a three-tier layout and propose the copy.", "-118m"),
      msg("sp_pricing", "Goose", "tool_request", "read_file docs/pricing.md", "-117m", "ap_done"),
      msg("sp_pricing", "Goose", "tool_result", "Read 84 lines. Tiers: Starter, Team, Enterprise.", "-116m"),
      msg("sp_pricing", "Goose", "agent", "Drafted the table. To apply it I need to write src/components/PricingTable.tsx — that's a repo change, so it needs your approval.", "-3m"),
      msg("sp_pricing", "Goose", "tool_request", "write_file src/components/PricingTable.tsx", "-2m", "ap_pending"),
    ],
    sp_migration: [
      msg("sp_migration", "Dan", "human", "Track the Postgres 16 upgrade here. First audit which queries use the deprecated syntax.", "-1d"),
      msg("sp_migration", "Goose", "agent", "On it. I'll grep the repo for the deprecated patterns and summarize by service.", "-1d"),
    ],
    sp_acme: [
      msg("sp_acme", "system", "ingress", "GitHub: Acme opened issue #412 \"SSO callback 500s on staging\".", "-5h"),
    ],
  };

  private approvals: Record<string, Approval[]> = {
    sp_pricing: [
      {
        id: "ap_pending",
        spaceId: "sp_pricing",
        agentRunId: "run_pricing",
        toolName: "write_file",
        toolInputJson: JSON.stringify(
          { path: "src/components/PricingTable.tsx", bytes: 2841 },
          null,
          2,
        ),
        status: "pending",
        requestedAt: rel("-2m"),
      },
    ],
  };

  private tasks: Record<string, Task[]> = {
    sp_pricing: [
      { id: "tk_table", spaceId: "sp_pricing", title: "Draft three-tier pricing table", state: "blocked_on_approval" },
      { id: "tk_copy", spaceId: "sp_pricing", title: "Write tier copy", state: "open" },
    ],
    sp_migration: [
      { id: "tk_audit", spaceId: "sp_migration", title: "Audit deprecated query syntax", state: "running" },
    ],
    sp_acme: [],
  };

  private runs: Record<string, AgentRun[]> = {
    sp_pricing: [
      { id: "run_pricing", spaceId: "sp_pricing", taskId: "tk_table", status: "waiting_approval", startedAt: rel("-118m"), endedAt: null },
    ],
    sp_migration: [
      { id: "run_mig", spaceId: "sp_migration", taskId: "tk_audit", status: "running", startedAt: rel("-1d"), endedAt: null },
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
    this.push(spaceId, msg(spaceId, "Priya", "human", prompt, "now"));
    this.push(spaceId, msg(spaceId, "Goose", "agent", "Picking this up — I'll work it in this space and pause for approval before any outside change.", "now"));
    this.notify(spaceId);
    return { kind: "ended", succeeded: true };
  }

  async decide(approvalId: string, status: ApprovalStatus): Promise<RunOutcome> {
    for (const spaceId of Object.keys(this.approvals)) {
      const ap = this.approvals[spaceId].find((a) => a.id === approvalId);
      if (!ap) continue;
      ap.status = status;
      const run = this.runs[spaceId]?.find((r) => r.id === ap.agentRunId);
      if (status === "approved") {
        this.push(spaceId, msg(spaceId, "Goose", "tool_result", `Applied ${ap.toolName}. Wrote ${JSON.parse(ap.toolInputJson).path}.`, "now"));
        this.push(spaceId, msg(spaceId, "Goose", "agent", "Done. The pricing table component is in place — want me to wire it into the route next?", "now"));
        if (run) run.status = "succeeded";
        this.setTaskState(spaceId, run?.taskId, "done");
      } else {
        this.push(spaceId, msg(spaceId, "Goose", "system", `Denied ${ap.toolName}. Holding — tell me what to do instead.`, "now"));
        if (run) run.status = "cancelled";
        this.setTaskState(spaceId, run?.taskId, "open");
      }
      this.notify(spaceId);
      return { kind: "ended", succeeded: status === "approved" };
    }
    return { kind: "ended", succeeded: false };
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
  authorName: string,
  kind: Message["kind"],
  body: string,
  when: string,
  refId: string | null = null,
): Message {
  return { id: id("m"), spaceId, authorName, kind, body, ts: rel(when), refId };
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
