import { ListTodo, Activity } from "lucide-react";
import type { AgentRun, Task } from "@/rpc/types";
import { Badge } from "@/components/ui/badge";
import { humanize, runStatusTone, taskStateTone } from "@/lib/labels";

interface Props {
  tasks: Task[];
  runs: AgentRun[];
}

export function TaskPanel({ tasks, runs }: Props) {
  return (
    <aside className="flex w-80 shrink-0 flex-col gap-6 overflow-y-auto border-l border-border bg-surface px-4 py-5">
      <section>
        <h2 className="mb-3 flex items-center gap-2 text-xs uppercase tracking-wide text-muted-foreground">
          <ListTodo className="size-4" /> Tasks
        </h2>
        <div className="space-y-2">
          {tasks.length === 0 && <p className="text-sm text-muted-foreground">No tasks yet.</p>}
          {tasks.map((t) => (
            <div
              key={t.id}
              className="rounded-lg border border-border bg-background/40 px-3 py-2"
            >
              <p className="text-sm leading-snug">{t.title}</p>
              <Badge tone={taskStateTone[t.state]} className="mt-2">
                {humanize(t.state)}
              </Badge>
            </div>
          ))}
        </div>
      </section>

      <section>
        <h2 className="mb-3 flex items-center gap-2 text-xs uppercase tracking-wide text-muted-foreground">
          <Activity className="size-4" /> Runs
        </h2>
        <div className="space-y-2">
          {runs.length === 0 && <p className="text-sm text-muted-foreground">No runs yet.</p>}
          {runs.map((r) => (
            <div
              key={r.id}
              className="flex items-center justify-between rounded-lg border border-border bg-background/40 px-3 py-2"
            >
              <span className="font-mono text-xs text-muted-foreground">{r.id}</span>
              <Badge tone={runStatusTone[r.status]}>{humanize(r.status)}</Badge>
            </div>
          ))}
        </div>
      </section>
    </aside>
  );
}
