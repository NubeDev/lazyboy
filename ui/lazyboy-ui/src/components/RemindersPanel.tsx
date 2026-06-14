import { useState } from "react";
import { BellRing } from "lucide-react";
import type { Reminder } from "@/rpc/types";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { absoluteTime, humanize, isOverdue, localInputToIso } from "@/lib/labels";

interface Props {
  reminders: Reminder[];
  now: number;
  onDismiss: (id: string) => Promise<void>;
  onCreate: (body: string, dueAtIso: string) => Promise<void>;
}

export function RemindersPanel({ reminders, now, onDismiss, onCreate }: Props) {
  const [body, setBody] = useState("");
  const [due, setDue] = useState("");
  const [busy, setBusy] = useState(false);

  const create = async () => {
    const text = body.trim();
    if (!text || !due || busy) return;
    setBusy(true);
    try {
      await onCreate(text, localInputToIso(due));
      setBody("");
      setDue("");
    } finally {
      setBusy(false);
    }
  };

  return (
    <section>
      <h2 className="mb-3 flex items-center gap-2 text-xs uppercase tracking-wide text-muted-foreground">
        <BellRing className="size-4" /> Reminders
      </h2>

      <div className="space-y-2">
        {reminders.length === 0 && (
          <p className="text-sm text-muted-foreground">No reminders.</p>
        )}
        {reminders.map((r) => {
          const overdue = r.status === "pending" && isOverdue(r.due_at, now);
          return (
            <div
              key={r.id}
              className="rounded-lg border border-border bg-background/40 px-3 py-2"
            >
              <p className="text-sm leading-snug">{r.body}</p>
              <div className="mt-2 flex items-center justify-between gap-2">
                <Badge tone={overdue ? "danger" : r.status === "pending" ? "neutral" : "success"}>
                  {overdue ? "overdue" : r.status === "pending" ? absoluteTime(r.due_at) : humanize(r.status)}
                </Badge>
                {r.status === "pending" && (
                  <Button size="sm" variant="ghost" onClick={() => void onDismiss(r.id)}>
                    Dismiss
                  </Button>
                )}
              </div>
            </div>
          );
        })}
      </div>

      <div className="mt-4 space-y-2">
        <input
          value={body}
          onChange={(e) => setBody(e.target.value)}
          placeholder="Remind me to…"
          className="w-full rounded-lg border border-border bg-surface px-3 py-2 text-sm outline-none placeholder:text-muted-foreground focus-visible:ring-2 focus-visible:ring-accent"
        />
        <input
          type="datetime-local"
          value={due}
          onChange={(e) => setDue(e.target.value)}
          className="w-full rounded-lg border border-border bg-surface px-3 py-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-accent"
        />
        <Button size="sm" disabled={busy || !body.trim() || !due} onClick={() => void create()}>
          Add reminder
        </Button>
      </div>
    </section>
  );
}
