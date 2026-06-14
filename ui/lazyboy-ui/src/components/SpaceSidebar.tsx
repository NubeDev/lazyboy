import { useState } from "react";
import { Hash, Plus, CircleDot, Settings2 } from "lucide-react";
import type { Health, Space } from "@/rpc/types";
import { cn } from "@/lib/utils";
import { GooseStatus } from "./GooseStatus";

interface Props {
  spaces: Space[];
  selectedId: string | null;
  pendingBySpace: Record<string, number>;
  health: Health | null;
  healthLoading: boolean;
  onSelect: (id: string) => void;
  onCreateSpace: (slug: string, title: string) => Promise<void>;
  onConfigureGoose: () => void;
}

// Derive a slug from the title as the user types, until they edit the
// slug field directly; once touched, the slug stops tracking the title so
// a deliberate slug is not clobbered.
function slugify(value: string): string {
  return value
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "");
}

export function SpaceSidebar({
  spaces,
  selectedId,
  pendingBySpace,
  health,
  healthLoading,
  onSelect,
  onCreateSpace,
  onConfigureGoose,
}: Props) {
  const [creating, setCreating] = useState(false);
  const [title, setTitle] = useState("");
  const [slug, setSlug] = useState("");
  const [slugTouched, setSlugTouched] = useState(false);

  const reset = () => {
    setCreating(false);
    setTitle("");
    setSlug("");
    setSlugTouched(false);
  };

  const submit = async () => {
    const finalSlug = (slugTouched ? slug : slugify(title)).trim();
    if (!finalSlug) return;
    await onCreateSpace(finalSlug, title.trim() || finalSlug);
    reset();
  };

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
        <button
          onClick={() => (creating ? reset() : setCreating(true))}
          className="text-muted-foreground hover:text-foreground"
          aria-label="New space"
          title="New space"
        >
          <Plus className="size-4" />
        </button>
      </div>

      {creating && (
        <form
          className="space-y-1.5 px-3 pb-2"
          onSubmit={(e) => {
            e.preventDefault();
            void submit();
          }}
        >
          <input
            autoFocus
            value={title}
            onChange={(e) => setTitle(e.target.value)}
            placeholder="Title"
            className="w-full rounded-md border border-border bg-background px-2 py-1 text-sm outline-none focus:border-accent"
          />
          <input
            value={slugTouched ? slug : slugify(title)}
            onChange={(e) => {
              setSlugTouched(true);
              setSlug(e.target.value);
            }}
            placeholder="slug"
            className="w-full rounded-md border border-border bg-background px-2 py-1 font-mono text-xs outline-none focus:border-accent"
          />
          <div className="flex gap-1.5">
            <button
              type="submit"
              className="flex-1 rounded-md bg-accent px-2 py-1 text-xs font-medium text-accent-foreground hover:opacity-90"
            >
              Create
            </button>
            <button
              type="button"
              onClick={reset}
              className="rounded-md border border-border px-2 py-1 text-xs text-muted-foreground hover:text-foreground"
            >
              Cancel
            </button>
          </div>
        </form>
      )}

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

      <div className="space-y-2 border-t border-border px-4 py-3">
        <div className="flex items-center justify-between gap-2">
          <GooseStatus health={health} loading={healthLoading} />
          <button
            onClick={onConfigureGoose}
            className="text-muted-foreground hover:text-foreground"
            aria-label="Configure goose provider"
            title="Configure goose provider"
          >
            <Settings2 className="size-4" />
          </button>
        </div>
        <p className="text-xs text-muted-foreground">one workspace · single trust boundary</p>
      </div>
    </aside>
  );
}
