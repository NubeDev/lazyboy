import { Hash, Plus, CircleDot } from "lucide-react";
import type { Space } from "@/rpc/types";
import { cn } from "@/lib/utils";

interface Props {
  spaces: Space[];
  selectedId: string | null;
  pendingBySpace: Record<string, number>;
  onSelect: (id: string) => void;
}

export function SpaceSidebar({ spaces, selectedId, pendingBySpace, onSelect }: Props) {
  return (
    <aside className="flex w-64 shrink-0 flex-col border-r border-border bg-surface">
      <div className="flex items-center gap-2 px-4 py-4">
        <div className="flex size-7 items-center justify-center rounded-lg bg-accent text-accent-foreground font-semibold">
          L
        </div>
        <span className="font-semibold tracking-tight">Lazyboy</span>
      </div>

      <div className="flex items-center justify-between px-4 pb-2 pt-3 text-xs uppercase tracking-wide text-muted-foreground">
        <span>Spaces</span>
        <Plus className="size-4 cursor-pointer hover:text-foreground" />
      </div>

      <nav className="flex-1 space-y-0.5 overflow-y-auto px-2">
        {spaces.map((s) => {
          const pending = pendingBySpace[s.id] ?? 0;
          const active = s.id === selectedId;
          return (
            <button
              key={s.id}
              onClick={() => onSelect(s.id)}
              className={cn(
                "flex w-full items-center gap-2 rounded-lg px-2 py-1.5 text-left text-sm transition-colors",
                active ? "bg-muted text-foreground" : "text-muted-foreground hover:bg-muted/60",
              )}
            >
              <Hash className="size-4 shrink-0 opacity-60" />
              <span className="truncate">{s.slug}</span>
              {pending > 0 && (
                <span className="ml-auto flex items-center gap-1 text-warning">
                  <CircleDot className="size-3" />
                  {pending}
                </span>
              )}
            </button>
          );
        })}
      </nav>

      <div className="border-t border-border px-4 py-3 text-xs text-muted-foreground">
        one workspace · single trust boundary
      </div>
    </aside>
  );
}
