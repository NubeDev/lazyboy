import { useCallback, useEffect, useMemo, useState } from "react";
import { useRpc } from "@/rpc/context";
import type { AgentRun, Approval, ApprovalStatus, Message, Space, Task } from "@/rpc/types";
import { SpaceSidebar } from "@/components/SpaceSidebar";
import { SpaceTimeline } from "@/components/SpaceTimeline";
import { TaskPanel } from "@/components/TaskPanel";

export function App() {
  const rpc = useRpc();
  const [spaces, setSpaces] = useState<Space[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [messages, setMessages] = useState<Message[]>([]);
  const [pending, setPending] = useState<Approval[]>([]);
  const [tasks, setTasks] = useState<Task[]>([]);
  const [runs, setRuns] = useState<AgentRun[]>([]);
  const [pendingBySpace, setPendingBySpace] = useState<Record<string, number>>({});

  // A fixed "now" reference for relative timestamps; ticked once a minute
  // so labels age without re-rendering on every frame.
  const [now, setNow] = useState(() => Date.parse("2026-06-13T14:05:00Z"));
  useEffect(() => {
    const t = setInterval(() => setNow((n) => n + 60_000), 60_000);
    return () => clearInterval(t);
  }, []);

  useEffect(() => {
    void rpc.listSpaces().then((s) => {
      setSpaces(s);
      setSelectedId((cur) => cur ?? s[0]?.id ?? null);
    });
  }, [rpc]);

  const refreshSpace = useCallback(
    async (spaceId: string) => {
      const [m, p, t, r] = await Promise.all([
        rpc.timeline(spaceId),
        rpc.listPending(spaceId),
        rpc.listTasks(spaceId),
        rpc.listRuns(spaceId),
      ]);
      setMessages(m);
      setPending(p);
      setTasks(t);
      setRuns(r);
    },
    [rpc],
  );

  useEffect(() => {
    void Promise.all(
      spaces.map((s) => rpc.listPending(s.id).then((p) => [s.id, p.length] as const)),
    ).then((entries) => setPendingBySpace(Object.fromEntries(entries)));
  }, [rpc, spaces, messages]);

  useEffect(() => {
    if (!selectedId) return;
    void refreshSpace(selectedId);
    return rpc.subscribe(selectedId, () => void refreshSpace(selectedId));
  }, [rpc, selectedId, refreshSpace]);

  const selectedSpace = useMemo(
    () => spaces.find((s) => s.id === selectedId) ?? null,
    [spaces, selectedId],
  );

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

  return (
    <div className="flex h-full">
      <SpaceSidebar
        spaces={spaces}
        selectedId={selectedId}
        pendingBySpace={pendingBySpace}
        onSelect={setSelectedId}
      />
      {selectedSpace ? (
        <>
          <SpaceTimeline
            space={selectedSpace}
            messages={messages}
            pending={pending}
            now={now}
            onSend={onSend}
            onDecide={onDecide}
          />
          <TaskPanel tasks={tasks} runs={runs} now={now} />
        </>
      ) : (
        <div className="flex flex-1 items-center justify-center text-muted-foreground">
          Select a space to begin.
        </div>
      )}
    </div>
  );
}
