import { useState } from "react";
import { GitCommitVertical } from "lucide-react";
import type { Decision } from "@/rpc/types";
import { Button } from "@/components/ui/button";
import { relativeTime } from "@/lib/labels";

interface Props {
  decisions: Decision[];
  now: number;
  onRecord: (summary: string) => Promise<void>;
}

export function DecisionsPanel({ decisions, now, onRecord }: Props) {
  const [draft, setDraft] = useState("");
  const [busy, setBusy] = useState(false);

  const record = async () => {
    const summary = draft.trim();
    if (!summary || busy) return;
    setBusy(true);
    setDraft("");
    try {
      await onRecord(summary);
    } finally {
      setBusy(false);
    }
  };

  return (
    <section>
      <h2 className="mb-3 flex items-center gap-2 text-xs uppercase tracking-wide text-muted-foreground">
        <GitCommitVertical className="size-4" /> Decisions
      </h2>

      <div className="space-y-2">
        {decisions.length === 0 && (
          <p className="text-sm text-muted-foreground">No decisions recorded.</p>
        )}
        {decisions.map((d) => (
          <div key={d.id} className="rounded-lg border border-border bg-background/40 px-3 py-2">
            <p className="text-sm leading-snug">{d.summary}</p>
            <p className="mt-1 text-xs text-muted-foreground">{relativeTime(d.decided_at, now)}</p>
          </div>
        ))}
      </div>

      <div className="mt-4 space-y-2">
        <textarea
          value={draft}
          onChange={(e) => setDraft(e.target.value)}
          rows={2}
          placeholder="Record a decision…"
          className="w-full resize-none rounded-lg border border-border bg-surface px-3 py-2 text-sm outline-none placeholder:text-muted-foreground focus-visible:ring-2 focus-visible:ring-accent"
        />
        <Button size="sm" disabled={busy || !draft.trim()} onClick={() => void record()}>
          Record
        </Button>
      </div>
    </section>
  );
}
