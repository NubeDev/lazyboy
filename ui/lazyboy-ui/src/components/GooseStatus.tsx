import { Loader2 } from "lucide-react";
import type { Health } from "@/rpc/types";
import { cn } from "@/lib/utils";

interface Props {
  health: Health | null;
  loading: boolean;
}

// The goose backend made visible: a dot whose colour tracks whether the
// node can reach `goose serve` right now. When goose is down the probe's
// failure reason rides the `title` so hovering explains why, not just
// that. `health == null && loading` is the first probe in flight.
export function GooseStatus({ health, loading }: Props) {
  const reachable = health?.goose_reachable ?? false;
  const label = !health && loading ? "Checking goose…" : reachable ? "Goose online" : "Goose offline";
  const tone = !health && loading ? "muted" : reachable ? "up" : "down";

  return (
    <div
      className="flex items-center gap-2 text-xs text-muted-foreground"
      title={health?.goose_detail ?? health?.goose_url ?? undefined}
    >
      {!health && loading ? (
        <Loader2 className="size-3 animate-spin" />
      ) : (
        <span
          className={cn(
            "size-2 rounded-full",
            tone === "up" && "bg-success",
            tone === "down" && "bg-danger",
            tone === "muted" && "bg-muted-foreground",
          )}
        />
      )}
      <span className="truncate">{label}</span>
    </div>
  );
}
