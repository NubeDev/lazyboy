import { useState } from "react";
import { Plug } from "lucide-react";
import type { Integration, Provider } from "@/rpc/types";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";

interface Props {
  integrations: Integration[];
  onCreate: (fields: {
    provider: Provider;
    account_ref: string;
    secret_ref: string;
  }) => Promise<void>;
}

const PROVIDERS: Provider[] = ["github", "gmail", "slack", "gcal"];

export function IntegrationsPanel({ integrations, onCreate }: Props) {
  const [provider, setProvider] = useState<Provider>("github");
  const [accountRef, setAccountRef] = useState("");
  const [secretRef, setSecretRef] = useState("");
  const [busy, setBusy] = useState(false);

  const create = async () => {
    if (!accountRef.trim() || !secretRef.trim() || busy) return;
    setBusy(true);
    try {
      await onCreate({
        provider,
        account_ref: accountRef.trim(),
        secret_ref: secretRef.trim(),
      });
      setAccountRef("");
      setSecretRef("");
    } finally {
      setBusy(false);
    }
  };

  return (
    <section>
      <h2 className="mb-3 flex items-center gap-2 text-xs uppercase tracking-wide text-muted-foreground">
        <Plug className="size-4" /> Integrations
      </h2>

      <div className="space-y-2">
        {integrations.length === 0 && (
          <p className="text-sm text-muted-foreground">No integrations connected.</p>
        )}
        {integrations.map((i) => (
          <div key={i.id} className="rounded-lg border border-border bg-background/40 px-3 py-2">
            <div className="flex items-center justify-between gap-2">
              <Badge tone="accent">{i.provider}</Badge>
              <Badge>{i.status}</Badge>
            </div>
            {i.account_ref && (
              <p className="mt-1 text-xs text-muted-foreground">{i.account_ref}</p>
            )}
          </div>
        ))}
      </div>

      <div className="mt-4 space-y-2">
        <select
          value={provider}
          onChange={(e) => setProvider(e.target.value as Provider)}
          className="w-full rounded-lg border border-border bg-surface px-3 py-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-accent"
        >
          {PROVIDERS.map((p) => (
            <option key={p} value={p}>
              {p}
            </option>
          ))}
        </select>
        <input
          value={accountRef}
          onChange={(e) => setAccountRef(e.target.value)}
          placeholder="Account reference"
          className="w-full rounded-lg border border-border bg-surface px-3 py-2 text-sm outline-none placeholder:text-muted-foreground focus-visible:ring-2 focus-visible:ring-accent"
        />
        <input
          value={secretRef}
          onChange={(e) => setSecretRef(e.target.value)}
          placeholder="Secret reference"
          className="w-full rounded-lg border border-border bg-surface px-3 py-2 text-sm outline-none placeholder:text-muted-foreground focus-visible:ring-2 focus-visible:ring-accent"
        />
        <Button
          size="sm"
          disabled={busy || !accountRef.trim() || !secretRef.trim()}
          onClick={() => void create()}
        >
          Connect
        </Button>
      </div>
    </section>
  );
}
