import { useState } from "react";
import { Workflow as WorkflowIcon, Play } from "lucide-react";
import type {
  ApprovalPolicy,
  TriggerKind,
  Workflow,
} from "@/rpc/types";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { humanize } from "@/lib/labels";

interface CreateFields {
  name: string;
  trigger_kind: TriggerKind;
  approval_policy: ApprovalPolicy;
  steps_json: string;
  trigger_config_json: string | null;
}

interface Props {
  workflows: Workflow[];
  onToggle: (workflow: Workflow) => Promise<void>;
  onFire: (workflow: Workflow) => Promise<void>;
  onCreate: (fields: CreateFields) => Promise<void>;
}

const TRIGGERS: TriggerKind[] = ["feed", "schedule"];
const POLICIES: ApprovalPolicy[] = ["require_approval", "auto_approve"];

export function WorkflowsPanel({ workflows, onToggle, onFire, onCreate }: Props) {
  const [name, setName] = useState("");
  const [trigger, setTrigger] = useState<TriggerKind>("feed");
  const [policy, setPolicy] = useState<ApprovalPolicy>("require_approval");
  const [steps, setSteps] = useState("");
  const [busy, setBusy] = useState(false);

  const create = async () => {
    const n = name.trim();
    const prompt = steps.trim();
    if (!n || !prompt || busy) return;
    setBusy(true);
    try {
      await onCreate({
        name: n,
        trigger_kind: trigger,
        approval_policy: policy,
        steps_json: JSON.stringify([{ prompt }]),
        trigger_config_json: null,
      });
      setName("");
      setSteps("");
    } finally {
      setBusy(false);
    }
  };

  return (
    <section>
      <h2 className="mb-3 flex items-center gap-2 text-xs uppercase tracking-wide text-muted-foreground">
        <WorkflowIcon className="size-4" /> Workflows
      </h2>

      <div className="space-y-2">
        {workflows.length === 0 && (
          <p className="text-sm text-muted-foreground">No workflows defined.</p>
        )}
        {workflows.map((w) => (
          <div key={w.id} className="rounded-lg border border-border bg-background/40 px-3 py-2">
            <div className="flex items-center justify-between gap-2">
              <p className="text-sm leading-snug">{w.name}</p>
              <Badge tone={w.status === "enabled" ? "success" : "neutral"}>
                {humanize(w.status)}
              </Badge>
            </div>
            <div className="mt-2 flex flex-wrap items-center gap-2">
              <Badge tone="accent">{humanize(w.trigger_kind)}</Badge>
              <Badge tone="warning">{humanize(w.approval_policy)}</Badge>
            </div>
            <div className="mt-2 flex gap-2">
              <Button size="sm" variant="outline" onClick={() => void onToggle(w)}>
                {w.status === "enabled" ? "Disable" : "Enable"}
              </Button>
              <Button size="sm" variant="ghost" onClick={() => void onFire(w)}>
                <Play /> Fire
              </Button>
            </div>
          </div>
        ))}
      </div>

      <div className="mt-4 space-y-2">
        <input
          value={name}
          onChange={(e) => setName(e.target.value)}
          placeholder="Workflow name"
          className="w-full rounded-lg border border-border bg-surface px-3 py-2 text-sm outline-none placeholder:text-muted-foreground focus-visible:ring-2 focus-visible:ring-accent"
        />
        <select
          value={trigger}
          onChange={(e) => setTrigger(e.target.value as TriggerKind)}
          className="w-full rounded-lg border border-border bg-surface px-3 py-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-accent"
        >
          {TRIGGERS.map((t) => (
            <option key={t} value={t}>
              {humanize(t)}
            </option>
          ))}
        </select>
        <select
          value={policy}
          onChange={(e) => setPolicy(e.target.value as ApprovalPolicy)}
          className="w-full rounded-lg border border-border bg-surface px-3 py-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-accent"
        >
          {POLICIES.map((p) => (
            <option key={p} value={p}>
              {humanize(p)}
            </option>
          ))}
        </select>
        <textarea
          value={steps}
          onChange={(e) => setSteps(e.target.value)}
          rows={2}
          placeholder="Prompt the workflow runs…"
          className="w-full resize-none rounded-lg border border-border bg-surface px-3 py-2 text-sm outline-none placeholder:text-muted-foreground focus-visible:ring-2 focus-visible:ring-accent"
        />
        <Button size="sm" disabled={busy || !name.trim() || !steps.trim()} onClick={() => void create()}>
          Create workflow
        </Button>
      </div>
    </section>
  );
}
