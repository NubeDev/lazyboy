import { useState, type ComponentType } from "react";
import { Plug, Workflow as WorkflowIcon } from "lucide-react";
import type {
  ApprovalPolicy,
  Integration,
  Provider,
  TriggerKind,
  Workflow,
} from "@/rpc/types";
import { cn } from "@/lib/utils";
import { IntegrationsPanel } from "./IntegrationsPanel";
import { WorkflowsPanel } from "./WorkflowsPanel";

interface WorkflowCreateFields {
  name: string;
  trigger_kind: TriggerKind;
  approval_policy: ApprovalPolicy;
  steps_json: string;
  trigger_config_json: string | null;
}

interface Props {
  integrations: Integration[];
  workflows: Workflow[];
  onCreateIntegration: (fields: {
    provider: Provider;
    account_ref: string;
    secret_ref: string;
  }) => Promise<void>;
  onToggleWorkflow: (workflow: Workflow) => Promise<void>;
  onFireWorkflow: (workflow: Workflow) => Promise<void>;
  onCreateWorkflow: (fields: WorkflowCreateFields) => Promise<void>;
}

type TabId = "integrations" | "workflows";

const TABS: { id: TabId; label: string; icon: ComponentType<{ className?: string }> }[] = [
  { id: "integrations", label: "Integrations", icon: Plug },
  { id: "workflows", label: "Workflows", icon: WorkflowIcon },
];

export function WorkspacePanels(props: Props) {
  const [tab, setTab] = useState<TabId>("integrations");

  return (
    <aside className="flex w-80 shrink-0 flex-col overflow-hidden border-l border-border bg-surface">
      <nav className="flex border-b border-border">
        {TABS.map(({ id, label, icon: Icon }) => (
          <button
            key={id}
            title={label}
            onClick={() => setTab(id)}
            className={cn(
              "flex flex-1 items-center justify-center py-3 text-muted-foreground transition-colors hover:bg-muted",
              tab === id && "border-b-2 border-accent text-foreground",
            )}
          >
            <Icon className="size-4" />
          </button>
        ))}
      </nav>

      <div className="flex-1 overflow-y-auto px-4 py-5">
        {tab === "integrations" && (
          <IntegrationsPanel
            integrations={props.integrations}
            onCreate={props.onCreateIntegration}
          />
        )}
        {tab === "workflows" && (
          <WorkflowsPanel
            workflows={props.workflows}
            onToggle={props.onToggleWorkflow}
            onFire={props.onFireWorkflow}
            onCreate={props.onCreateWorkflow}
          />
        )}
      </div>
    </aside>
  );
}
