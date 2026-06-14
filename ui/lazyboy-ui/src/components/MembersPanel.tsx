import { useState } from "react";
import { Users } from "lucide-react";
import type { Group, Membership } from "@/rpc/types";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";

type PrincipalKind = "user" | "group";

interface Props {
  members: Membership[];
  groups: Group[];
  onGrant: (principalKind: PrincipalKind, principalId: string, role: string) => Promise<void>;
  onCreateGroup: (name: string) => Promise<void>;
}

export function MembersPanel({ members, groups, onGrant, onCreateGroup }: Props) {
  const [kind, setKind] = useState<PrincipalKind>("user");
  const [principalId, setPrincipalId] = useState("");
  const [role, setRole] = useState("member");
  const [groupName, setGroupName] = useState("");
  const [busy, setBusy] = useState(false);

  const grant = async () => {
    const id = principalId.trim();
    const r = role.trim() || "member";
    if (!id || busy) return;
    setBusy(true);
    try {
      await onGrant(kind, id, r);
      setPrincipalId("");
    } finally {
      setBusy(false);
    }
  };

  const createGroup = async () => {
    const n = groupName.trim();
    if (!n || busy) return;
    setBusy(true);
    try {
      await onCreateGroup(n);
      setGroupName("");
    } finally {
      setBusy(false);
    }
  };

  return (
    <section className="space-y-6">
      <div>
        <h2 className="mb-3 flex items-center gap-2 text-xs uppercase tracking-wide text-muted-foreground">
          <Users className="size-4" /> Grant membership
        </h2>
        <div className="space-y-2">
          <select
            value={kind}
            onChange={(e) => setKind(e.target.value as PrincipalKind)}
            className="w-full rounded-lg border border-border bg-surface px-3 py-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-accent"
          >
            <option value="user">user</option>
            <option value="group">group</option>
          </select>
          <input
            value={principalId}
            onChange={(e) => setPrincipalId(e.target.value)}
            placeholder="Principal id"
            className="w-full rounded-lg border border-border bg-surface px-3 py-2 text-sm outline-none placeholder:text-muted-foreground focus-visible:ring-2 focus-visible:ring-accent"
          />
          <input
            value={role}
            onChange={(e) => setRole(e.target.value)}
            placeholder="Role"
            className="w-full rounded-lg border border-border bg-surface px-3 py-2 text-sm outline-none placeholder:text-muted-foreground focus-visible:ring-2 focus-visible:ring-accent"
          />
          <Button size="sm" disabled={busy || !principalId.trim()} onClick={() => void grant()}>
            Grant
          </Button>
        </div>
      </div>

      <div>
        <h2 className="mb-3 text-xs uppercase tracking-wide text-muted-foreground">Members</h2>
        <div className="space-y-2">
          {members.length === 0 && <p className="text-sm text-muted-foreground">No members yet.</p>}
          {members.map((m) => (
            <div
              key={m.id}
              className="flex items-center justify-between rounded-lg border border-border bg-background/40 px-3 py-2"
            >
              <div className="min-w-0">
                <p className="truncate text-sm leading-snug">{m.principal_id}</p>
                <p className="text-xs text-muted-foreground">{m.principal_kind}</p>
              </div>
              <Badge tone="accent">{m.role}</Badge>
            </div>
          ))}
        </div>
      </div>

      <div>
        <h2 className="mb-3 text-xs uppercase tracking-wide text-muted-foreground">Groups</h2>
        <div className="space-y-2">
          {groups.length === 0 && (
            <p className="text-sm text-muted-foreground">No groups.</p>
          )}
          {groups.map((g) => (
            <div key={g.id} className="rounded-lg border border-border bg-background/40 px-3 py-2">
              <p className="text-sm leading-snug">{g.name}</p>
            </div>
          ))}
        </div>
        <div className="mt-4 space-y-2">
          <input
            value={groupName}
            onChange={(e) => setGroupName(e.target.value)}
            placeholder="New group name"
            className="w-full rounded-lg border border-border bg-surface px-3 py-2 text-sm outline-none placeholder:text-muted-foreground focus-visible:ring-2 focus-visible:ring-accent"
          />
          <Button size="sm" variant="outline" disabled={busy || !groupName.trim()} onClick={() => void createGroup()}>
            Create group
          </Button>
        </div>
      </div>
    </section>
  );
}
