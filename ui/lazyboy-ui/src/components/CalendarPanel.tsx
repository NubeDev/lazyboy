import { useState } from "react";
import { CalendarDays } from "lucide-react";
import type { CalendarEvent, UpsertCalendarBody } from "@/rpc/types";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { absoluteTime, localInputToIso } from "@/lib/labels";

interface Props {
  calendar: CalendarEvent[];
  onCreate: (body: UpsertCalendarBody) => Promise<void>;
}

export function CalendarPanel({ calendar, onCreate }: Props) {
  const [title, setTitle] = useState("");
  const [start, setStart] = useState("");
  const [end, setEnd] = useState("");
  const [busy, setBusy] = useState(false);

  const sorted = [...calendar].sort(
    (a, b) => Date.parse(a.starts_at) - Date.parse(b.starts_at),
  );

  const create = async () => {
    const t = title.trim();
    if (!t || !start || busy) return;
    setBusy(true);
    try {
      await onCreate({
        source: "local",
        title: t,
        starts_at: localInputToIso(start),
        ends_at: end ? localInputToIso(end) : null,
      });
      setTitle("");
      setStart("");
      setEnd("");
    } finally {
      setBusy(false);
    }
  };

  return (
    <section>
      <h2 className="mb-3 flex items-center gap-2 text-xs uppercase tracking-wide text-muted-foreground">
        <CalendarDays className="size-4" /> Calendar
      </h2>

      <div className="space-y-2">
        {sorted.length === 0 && (
          <p className="text-sm text-muted-foreground">No events.</p>
        )}
        {sorted.map((e) => (
          <div key={e.id} className="rounded-lg border border-border bg-background/40 px-3 py-2">
            <div className="flex items-center justify-between gap-2">
              <p className="text-sm leading-snug">{e.title}</p>
              <Badge>{e.source}</Badge>
            </div>
            <p className="mt-1 text-xs text-muted-foreground">
              {absoluteTime(e.starts_at)}
              {e.ends_at ? ` – ${absoluteTime(e.ends_at)}` : ""}
            </p>
          </div>
        ))}
      </div>

      <div className="mt-4 space-y-2">
        <input
          value={title}
          onChange={(e) => setTitle(e.target.value)}
          placeholder="Event title"
          className="w-full rounded-lg border border-border bg-surface px-3 py-2 text-sm outline-none placeholder:text-muted-foreground focus-visible:ring-2 focus-visible:ring-accent"
        />
        <label className="block text-xs text-muted-foreground">
          Starts
          <input
            type="datetime-local"
            value={start}
            onChange={(e) => setStart(e.target.value)}
            className="mt-1 w-full rounded-lg border border-border bg-surface px-3 py-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-accent"
          />
        </label>
        <label className="block text-xs text-muted-foreground">
          Ends (optional)
          <input
            type="datetime-local"
            value={end}
            onChange={(e) => setEnd(e.target.value)}
            className="mt-1 w-full rounded-lg border border-border bg-surface px-3 py-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-accent"
          />
        </label>
        <Button size="sm" disabled={busy || !title.trim() || !start} onClick={() => void create()}>
          Add event
        </Button>
      </div>
    </section>
  );
}
