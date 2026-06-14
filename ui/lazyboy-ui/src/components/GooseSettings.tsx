import { useEffect, useMemo, useState } from "react";
import { X, Check, Loader2, KeyRound } from "lucide-react";
import { useRpc } from "@/rpc/context";
import type { GooseConfig, GooseProvider } from "@/rpc/types";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";

interface Props {
  onClose: () => void;
  // Called after a successful save so the parent can re-probe goose health.
  onSaved: () => void;
}

// The goose provider manager: pick a provider and model, set its API key,
// and apply — which relaunches goose under the new provider. The key is
// write-only here: the backend reports whether one is stored (`key_set`),
// never the value (SCOPE R5), so an existing key shows as set and is left
// untouched unless the user types a replacement.
export function GooseSettings({ onClose, onSaved }: Props) {
  const rpc = useRpc();
  const [providers, setProviders] = useState<GooseProvider[]>([]);
  const [config, setConfig] = useState<GooseConfig | null>(null);
  const [loading, setLoading] = useState(true);
  const [providerId, setProviderId] = useState("");
  const [model, setModel] = useState("");
  const [apiKey, setApiKey] = useState("");
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    void Promise.all([rpc.listGooseProviders(), rpc.getGooseConfig()])
      .then(([ps, cfg]) => {
        if (cancelled) return;
        setProviders(ps);
        setConfig(cfg);
        setProviderId(cfg.provider ?? ps[0]?.id ?? "");
        setModel(cfg.model ?? "");
      })
      .catch((e) => !cancelled && setError(errMsg(e)))
      .finally(() => !cancelled && setLoading(false));
    return () => {
      cancelled = true;
    };
  }, [rpc]);

  const selected = useMemo(
    () => providers.find((p) => p.id === providerId) ?? null,
    [providers, providerId],
  );

  // A provider switch resets the model to that provider's first suggestion
  // unless the current selection already lists the model.
  const onPickProvider = (id: string) => {
    setProviderId(id);
    const p = providers.find((x) => x.id === id);
    if (p && !p.models.includes(model)) setModel(p.models[0] ?? "");
    setApiKey("");
  };

  const needsKey = !!selected?.requires_key && !selected.key_set && apiKey.trim() === "";

  const save = async () => {
    if (!providerId || saving || needsKey) return;
    setSaving(true);
    setError(null);
    try {
      const cfg = await rpc.setGooseConfig({
        provider: providerId,
        model: model.trim() || null,
        // Omit when blank so an existing key is preserved; a typed value
        // replaces it.
        api_key: apiKey.trim() === "" ? undefined : apiKey.trim(),
      });
      setConfig(cfg);
      setApiKey("");
      // Reflect the now-stored key in the provider list.
      setProviders((ps) =>
        ps.map((p) => (p.id === providerId ? { ...p, key_set: p.key_set || apiKey.trim() !== "" } : p)),
      );
      onSaved();
    } catch (e) {
      setError(errMsg(e));
    } finally {
      setSaving(false);
    }
  };

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-4"
      onClick={onClose}
    >
      <div
        className="w-full max-w-md rounded-[var(--radius-card)] border border-border bg-surface shadow-xl"
        onClick={(e) => e.stopPropagation()}
      >
        <header className="flex items-center justify-between border-b border-border px-5 py-4">
          <h2 className="text-sm font-semibold">Goose AI provider</h2>
          <Button size="icon" variant="ghost" onClick={onClose} aria-label="Close">
            <X />
          </Button>
        </header>

        {loading ? (
          <div className="flex items-center justify-center gap-2 px-5 py-12 text-sm text-muted-foreground">
            <Loader2 className="size-4 animate-spin" /> Loading…
          </div>
        ) : (
          <div className="space-y-4 px-5 py-5">
            {config && (
              <div className="flex items-center gap-2 text-xs text-muted-foreground">
                <span>Status:</span>
                {config.running ? (
                  <Badge tone="success">running</Badge>
                ) : (
                  <Badge tone="danger">stopped</Badge>
                )}
                {config.provider && <span>· {config.provider}</span>}
              </div>
            )}

            <label className="block space-y-1">
              <span className="text-xs uppercase tracking-wide text-muted-foreground">Provider</span>
              <select
                value={providerId}
                onChange={(e) => onPickProvider(e.target.value)}
                className="w-full rounded-lg border border-border bg-background px-3 py-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-accent"
              >
                {providers.map((p) => (
                  <option key={p.id} value={p.id}>
                    {p.display_name}
                  </option>
                ))}
              </select>
            </label>

            <label className="block space-y-1">
              <span className="text-xs uppercase tracking-wide text-muted-foreground">Model</span>
              <input
                value={model}
                onChange={(e) => setModel(e.target.value)}
                list="goose-models"
                placeholder="provider default"
                className="w-full rounded-lg border border-border bg-background px-3 py-2 text-sm outline-none placeholder:text-muted-foreground focus-visible:ring-2 focus-visible:ring-accent"
              />
              <datalist id="goose-models">
                {selected?.models.map((m) => (
                  <option key={m} value={m} />
                ))}
              </datalist>
            </label>

            {selected?.requires_key && (
              <label className="block space-y-1">
                <span className="flex items-center gap-2 text-xs uppercase tracking-wide text-muted-foreground">
                  <KeyRound className="size-3" /> API key
                  {selected.key_set && (
                    <Badge tone="success" className="normal-case">
                      set
                    </Badge>
                  )}
                </span>
                <input
                  type="password"
                  value={apiKey}
                  onChange={(e) => setApiKey(e.target.value)}
                  placeholder={selected.key_set ? "leave blank to keep current" : "paste key"}
                  className="w-full rounded-lg border border-border bg-background px-3 py-2 text-sm outline-none placeholder:text-muted-foreground focus-visible:ring-2 focus-visible:ring-accent"
                />
              </label>
            )}

            {error && (
              <p className="rounded-lg border border-danger/40 bg-danger/5 px-3 py-2 text-xs text-danger">
                {error}
              </p>
            )}

            <div className="flex justify-end gap-2 pt-1">
              <Button variant="outline" size="sm" onClick={onClose}>
                Cancel
              </Button>
              <Button size="sm" disabled={saving || needsKey || !providerId} onClick={() => void save()}>
                {saving ? <Loader2 className="animate-spin" /> : <Check />}
                Save &amp; restart goose
              </Button>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

function errMsg(e: unknown): string {
  return e instanceof Error ? e.message : String(e);
}
