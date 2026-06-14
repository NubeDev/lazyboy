import { useCallback, useEffect, useMemo, useState } from "react";
import { Plug2, X } from "lucide-react";
import { useRpc } from "@/rpc/context";
import type {
  AgentRun,
  Approval,
  ApprovalStatus,
  CalendarEvent,
  Decision,
  Group,
  Health,
  Integration,
  Membership,
  Message,
  Provider,
  Reminder,
  Space,
  Task,
  Workflow,
} from "@/rpc/types";
import { SpaceSidebar } from "@/components/SpaceSidebar";
import { GooseSettings } from "@/components/GooseSettings";
import { SpaceTimeline } from "@/components/SpaceTimeline";
import { SpacePanels } from "@/components/SpacePanels";
import { WorkspacePanels } from "@/components/WorkspacePanels";
import { Button } from "@/components/ui/button";

type PrincipalKind = "user" | "group";

interface WorkflowCreateFields {
  name: string;
  trigger_kind: Workflow["trigger_kind"];
  approval_policy: Workflow["approval_policy"];
  steps_json: string;
  trigger_config_json: string | null;
}

function errorMessage(e: unknown): string {
  return e instanceof Error ? e.message : String(e);
}

export function App() {
  const rpc = useRpc();
  const [spaces, setSpaces] = useState<Space[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [messages, setMessages] = useState<Message[]>([]);
  const [pending, setPending] = useState<Approval[]>([]);
  const [tasks, setTasks] = useState<Task[]>([]);
  const [runs, setRuns] = useState<AgentRun[]>([]);
  const [decisions, setDecisions] = useState<Decision[]>([]);
  const [reminders, setReminders] = useState<Reminder[]>([]);
  const [calendar, setCalendar] = useState<CalendarEvent[]>([]);
  const [integrations, setIntegrations] = useState<Integration[]>([]);
  const [workflows, setWorkflows] = useState<Workflow[]>([]);
  const [groups, setGroups] = useState<Group[]>([]);
  const [members, setMembers] = useState<Membership[]>([]);
  const [pendingBySpace, setPendingBySpace] = useState<Record<string, number>>({});
  const [showWorkspace, setShowWorkspace] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [health, setHealth] = useState<Health | null>(null);
  const [healthLoading, setHealthLoading] = useState(true);
  const [gooseSettingsOpen, setGooseSettingsOpen] = useState(false);

  // One goose reachability probe. A rejected probe is a transport failure
  // to the node itself (not goose being down, which the endpoint reports
  // as a 200 body), so it reads as offline rather than hitting the error
  // banner. Shared by the poll loop and the post-save refresh.
  const probeHealth = useCallback(() => {
    rpc
      .health()
      .then(setHealth)
      .catch(() =>
        setHealth({ goose_url: "", goose_reachable: false, goose_detail: "core unreachable" }),
      )
      .finally(() => setHealthLoading(false));
  }, [rpc]);

  // Wrap a mutation so a rejected RPC surfaces in the banner instead of
  // becoming an unhandled rejection.
  const guard = useCallback(
    <A extends unknown[]>(fn: (...args: A) => Promise<void>) =>
      async (...args: A): Promise<void> => {
        try {
          await fn(...args);
        } catch (e) {
          setError(errorMessage(e));
        }
      },
    [],
  );

  // A live "now" reference for relative timestamps; ticked once a minute
  // so labels age without re-rendering on every frame. Anchored to the
  // wall clock (not a fixed instant) so overdue reminders read correctly.
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    const t = setInterval(() => setNow(Date.now()), 60_000);
    return () => clearInterval(t);
  }, []);

  // Poll goose reachability for the sidebar status.
  useEffect(() => {
    probeHealth();
    const t = setInterval(probeHealth, 15_000);
    return () => clearInterval(t);
  }, [probeHealth]);

  useEffect(() => {
    void rpc
      .listSpaces()
      .then((s) => {
        setSpaces(s);
        setSelectedId((cur) => cur ?? s[0]?.id ?? null);
      })
      .catch((e) => setError(errorMessage(e)));
  }, [rpc]);

  const onCreateSpace = useCallback(
    async (slug: string, title: string) => {
      const created = await rpc.createSpace(slug, title);
      setSpaces((cur) => [...cur, created]);
      setSelectedId(created.id);
    },
    [rpc],
  );

  const selectedSpace = useMemo(
    () => spaces.find((s) => s.id === selectedId) ?? null,
    [spaces, selectedId],
  );
  const workspaceId = selectedSpace?.workspace_id ?? null;

  const refreshSpace = useCallback(
    async (spaceId: string) => {
      const [m, p, t, r, d, rem, cal, mem] = await Promise.all([
        rpc.timeline(spaceId),
        rpc.listPending(spaceId),
        rpc.listTasks(spaceId),
        rpc.listRuns(spaceId),
        rpc.listDecisions(spaceId),
        rpc.listReminders(spaceId),
        rpc.listCalendar(spaceId),
        rpc.listMembers(spaceId),
      ]);
      setMessages(m);
      setPending(p);
      setTasks(t);
      setRuns(r);
      setDecisions(d);
      setReminders(rem);
      setCalendar(cal);
      setMembers(mem);
      // Reuse the pending list just fetched so resolving an approval clears
      // the sidebar badge without a full all-spaces sweep.
      setPendingBySpace((cur) => ({ ...cur, [spaceId]: p.length }));
    },
    [rpc],
  );

  const refreshWorkspace = useCallback(
    async (wsId: string) => {
      const [ints, wfs] = await Promise.all([
        rpc.listIntegrations(wsId),
        rpc.listWorkflows(wsId),
      ]);
      setIntegrations(ints);
      setWorkflows(wfs);
    },
    [rpc],
  );

  // Full sweep only when the space list itself changes; per-space counts are
  // kept current by refreshSpace afterwards.
  useEffect(() => {
    void Promise.all(
      spaces.map((s) => rpc.listPending(s.id).then((p) => [s.id, p.length] as const)),
    )
      .then((entries) => setPendingBySpace(Object.fromEntries(entries)))
      .catch((e) => setError(errorMessage(e)));
  }, [rpc, spaces]);

  useEffect(() => {
    if (!selectedId) return;
    void refreshSpace(selectedId).catch((e) => setError(errorMessage(e)));
    return rpc.subscribe(
      selectedId,
      () => void refreshSpace(selectedId).catch((e) => setError(errorMessage(e))),
    );
  }, [rpc, selectedId, refreshSpace]);

  useEffect(() => {
    // Groups are workspace-scoped and only appended to locally; clear them so
    // a stale list from a prior workspace does not bleed across switches.
    setGroups([]);
    if (!workspaceId) return;
    void refreshWorkspace(workspaceId).catch((e) => setError(errorMessage(e)));
  }, [workspaceId, refreshWorkspace]);

  const onSend = useCallback(
    async (prompt: string) => {
      if (!selectedId) return;
      await rpc.startRun(selectedId, prompt);
      await refreshSpace(selectedId);
    },
    [rpc, selectedId, refreshSpace],
  );

  const onDecide = useCallback(
    async (approvalId: string, status: ApprovalStatus) => {
      await rpc.decide(approvalId, status);
      if (selectedId) await refreshSpace(selectedId);
    },
    [rpc, selectedId, refreshSpace],
  );

  const onRecordDecision = useCallback(
    async (summary: string) => {
      if (!selectedId) return;
      await rpc.recordDecision(selectedId, { summary });
      await refreshSpace(selectedId);
    },
    [rpc, selectedId, refreshSpace],
  );

  const onCreateReminder = useCallback(
    async (body: string, dueAtIso: string) => {
      if (!selectedId) return;
      await rpc.createReminder(selectedId, { body, due_at: dueAtIso });
      await refreshSpace(selectedId);
    },
    [rpc, selectedId, refreshSpace],
  );

  const onDismissReminder = useCallback(
    async (id: string) => {
      await rpc.dismissReminder(id);
      if (selectedId) await refreshSpace(selectedId);
    },
    [rpc, selectedId, refreshSpace],
  );

  const onCreateCalendar = useCallback(
    async (body: Parameters<typeof rpc.upsertCalendar>[1]) => {
      if (!selectedId) return;
      await rpc.upsertCalendar(selectedId, body);
      await refreshSpace(selectedId);
    },
    [rpc, selectedId, refreshSpace],
  );

  const onGrantMembership = useCallback(
    async (principalKind: PrincipalKind, principalId: string, role: string) => {
      if (!selectedId) return;
      await rpc.grantMembership(selectedId, principalKind, principalId, role);
      await refreshSpace(selectedId);
    },
    [rpc, selectedId, refreshSpace],
  );

  const onCreateGroup = useCallback(
    async (name: string) => {
      if (!workspaceId) return;
      const g = await rpc.createGroup(workspaceId, name);
      setGroups((cur) => [...cur, g]);
    },
    [rpc, workspaceId],
  );

  const onCreateIntegration = useCallback(
    async (fields: { provider: Provider; account_ref: string; secret_ref: string }) => {
      if (!workspaceId) return;
      await rpc.createIntegration({ workspace_id: workspaceId, ...fields });
      await refreshWorkspace(workspaceId);
    },
    [rpc, workspaceId, refreshWorkspace],
  );

  const onToggleWorkflow = useCallback(
    async (workflow: Workflow) => {
      if (workflow.status === "enabled") await rpc.disableWorkflow(workflow.id);
      else await rpc.enableWorkflow(workflow.id);
      if (workspaceId) await refreshWorkspace(workspaceId);
    },
    [rpc, workspaceId, refreshWorkspace],
  );

  const onFireWorkflow = useCallback(
    async (workflow: Workflow) => {
      if (!selectedId) return;
      await rpc.fireWorkflow(workflow.id, selectedId);
      await refreshSpace(selectedId);
    },
    [rpc, selectedId, refreshSpace],
  );

  const onCreateWorkflow = useCallback(
    async (fields: WorkflowCreateFields) => {
      if (!workspaceId) return;
      await rpc.createWorkflow({ workspace_id: workspaceId, ...fields });
      await refreshWorkspace(workspaceId);
    },
    [rpc, workspaceId, refreshWorkspace],
  );

  return (
    <div className="flex h-full">
      <SpaceSidebar
        spaces={spaces}
        selectedId={selectedId}
        pendingBySpace={pendingBySpace}
        health={health}
        healthLoading={healthLoading}
        onSelect={setSelectedId}
        onCreateSpace={guard(onCreateSpace)}
        onConfigureGoose={() => setGooseSettingsOpen(true)}
      />
      {selectedSpace ? (
        <>
          <div className="flex h-full min-h-0 min-w-0 flex-1 flex-col">
            {error && (
              <div className="flex items-center justify-between gap-2 border-b border-danger/40 bg-danger/20 px-4 py-2 text-sm text-danger">
                <span className="min-w-0 truncate">{error}</span>
                <Button
                  size="icon"
                  variant="ghost"
                  aria-label="Dismiss error"
                  onClick={() => setError(null)}
                >
                  <X />
                </Button>
              </div>
            )}
            <div className="flex items-center justify-end border-b border-border px-4 py-2">
              <Button
                size="sm"
                variant={showWorkspace ? "default" : "outline"}
                onClick={() => setShowWorkspace((v) => !v)}
              >
                <Plug2 /> Workspace
              </Button>
            </div>
            <SpaceTimeline
              space={selectedSpace}
              messages={messages}
              pending={pending}
              now={now}
              onSend={guard(onSend)}
              onCreateReminder={guard(onCreateReminder)}
              onDecide={guard(onDecide)}
            />
          </div>
          <SpacePanels
            tasks={tasks}
            runs={runs}
            decisions={decisions}
            reminders={reminders}
            calendar={calendar}
            groups={groups}
            members={members}
            now={now}
            onRecordDecision={guard(onRecordDecision)}
            onDismissReminder={guard(onDismissReminder)}
            onCreateReminder={guard(onCreateReminder)}
            onCreateCalendar={guard(onCreateCalendar)}
            onGrantMembership={guard(onGrantMembership)}
            onCreateGroup={guard(onCreateGroup)}
          />
          {showWorkspace && (
            <WorkspacePanels
              integrations={integrations}
              workflows={workflows}
              onCreateIntegration={guard(onCreateIntegration)}
              onToggleWorkflow={guard(onToggleWorkflow)}
              onFireWorkflow={guard(onFireWorkflow)}
              onCreateWorkflow={guard(onCreateWorkflow)}
            />
          )}
        </>
      ) : (
        <div className="flex flex-1 items-center justify-center text-muted-foreground">
          Select a space to begin.
        </div>
      )}
      {gooseSettingsOpen && (
        <GooseSettings
          onClose={() => setGooseSettingsOpen(false)}
          onSaved={probeHealth}
        />
      )}
    </div>
  );
}
