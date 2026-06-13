import { useState } from "react";
import { ShieldAlert, Check, X } from "lucide-react";
import type { Approval, ApprovalStatus } from "@/rpc/types";
import { Card } from "@/components/ui/card";
import { Button } from "@/components/ui/button";

interface Props {
  approval: Approval;
  onDecide: (status: ApprovalStatus) => Promise<void>;
}

// The trust layer made visible: a tool request gates an outside-world
// change and waits inline in the timeline until a human resolves it.
export function ApprovalCard({ approval, onDecide }: Props) {
  const [busy, setBusy] = useState(false);

  const decide = async (status: ApprovalStatus) => {
    setBusy(true);
    try {
      await onDecide(status);
    } finally {
      setBusy(false);
    }
  };

  return (
    <Card className="border-warning/40 bg-warning/5 p-4">
      <div className="flex items-center gap-2 text-warning">
        <ShieldAlert className="size-4" />
        <span className="text-sm font-medium">Approval needed</span>
      </div>
      <p className="mt-2 text-sm text-foreground">
        Goose wants to run{" "}
        <code className="rounded bg-muted px-1.5 py-0.5 text-xs">{approval.toolName}</code>
      </p>
      <pre className="mt-3 max-h-48 overflow-auto rounded-lg bg-background/60 p-3 text-xs text-muted-foreground">
        {approval.toolInputJson}
      </pre>
      <div className="mt-3 flex gap-2">
        <Button size="sm" variant="success" disabled={busy} onClick={() => decide("approved")}>
          <Check /> Approve
        </Button>
        <Button size="sm" variant="danger" disabled={busy} onClick={() => decide("denied")}>
          <X /> Deny
        </Button>
      </div>
    </Card>
  );
}
