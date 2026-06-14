import { useEffect, useRef, useState } from "react";
import { Send, Hash } from "lucide-react";
import type { Approval, ApprovalStatus, Message, Space } from "@/rpc/types";
import { TimelineMessage } from "./TimelineMessage";
import { ApprovalCard } from "./ApprovalCard";
import { Button } from "@/components/ui/button";

interface Props {
  space: Space;
  messages: Message[];
  pending: Approval[];
  now: number;
  // Natural-language send: routed to the agent (which acts via its
  // lazyboy tools). Slash commands below short-circuit to deterministic
  // RPCs and never reach the agent.
  onSend: (prompt: string) => Promise<void>;
  onCreateReminder: (body: string, dueAtIso: string) => Promise<void>;
  onDecide: (approvalId: string, status: ApprovalStatus) => Promise<void>;
}

// Prompt chips: one tap fills the input with a starting point the user
// can edit before sending — the "prompts like Slack" affordance. The
// overview/needs-me chips are natural language (the agent answers via
// `space_overview`); the task chip seeds the deterministic `/task`.
const CHIPS: { label: string; fill: string }[] = [
  { label: "Give me an overview", fill: "Give me an overview of this space." },
  { label: "Add a task", fill: "/task " },
  { label: "What needs me?", fill: "What in this space is waiting on me right now?" },
];

const HELP = [
  "/task <what you want>   the agent adds a task with a clean title",
  "/remind <2h|1d|…> <text>   set a reminder",
  "/overview            ask the agent to summarise this space",
  "/help                show this list",
  "",
  "Anything else is sent to the agent.",
].join("\n");

/// Turn a `/remind` argument into an ISO due-time and body. A leading
/// relative token (`30m`, `2h`, `1d`) sets the offset from now; without
/// one the whole argument is the body and the reminder defaults to a day
/// out, so a hurried `/remind call Sam` still lands somewhere sensible.
function parseRemind(arg: string): { dueIso: string; body: string } {
  const m = arg.match(/^(\d+)([mhd])\s+(.*)$/);
  const unitMs = { m: 60_000, h: 3_600_000, d: 86_400_000 };
  if (m) {
    const offset = Number(m[1]) * unitMs[m[2] as "m" | "h" | "d"];
    return { dueIso: new Date(Date.now() + offset).toISOString(), body: m[3].trim() };
  }
  return { dueIso: new Date(Date.now() + unitMs.d).toISOString(), body: arg };
}

export function SpaceTimeline({
  space,
  messages,
  pending,
  now,
  onSend,
  onCreateReminder,
  onDecide,
}: Props) {
  const [draft, setDraft] = useState("");
  const [sending, setSending] = useState(false);
  const [hint, setHint] = useState<string | null>(null);
  const endRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLTextAreaElement>(null);
  const pendingByRef = new Map(pending.map((a) => [a.id, a]));

  useEffect(() => {
    endRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages.length]);

  // Dispatch one line: slash commands resolve deterministically, anything
  // else goes to the agent. A usage error stays inline (the `hint`) rather
  // than bubbling to the app-level error banner.
  const dispatch = async (text: string): Promise<void> => {
    if (!text.startsWith("/")) {
      await onSend(text);
      return;
    }
    const cmd = text.split(/\s+/, 1)[0];
    const arg = text.slice(cmd.length).trim();
    switch (cmd) {
      case "/task":
        if (!arg) throw new Error("usage: /task <what you want done>");
        // Route through the agent so it extracts a clean title from
        // however the user phrased it ("add me a task to do homewor" ->
        // "Do homework"), rather than using the raw text verbatim.
        await onSend(`Create a task in this space: ${arg}`);
        return;
      case "/remind": {
        if (!arg) throw new Error("usage: /remind <2h|1d|…> <text>");
        const { dueIso, body } = parseRemind(arg);
        if (!body) throw new Error("usage: /remind <2h|1d|…> <text>");
        await onCreateReminder(body, dueIso);
        return;
      }
      case "/overview":
        await onSend("Give me an overview of this space: open tasks, recent activity, and anything waiting on me.");
        return;
      case "/help":
        setHint(HELP);
        return;
      default:
        throw new Error(`unknown command ${cmd} — try /help`);
    }
  };

  const send = async () => {
    const text = draft.trim();
    if (!text || sending) return;
    setSending(true);
    setHint(null);
    const restore = draft;
    setDraft("");
    try {
      await dispatch(text);
    } catch (e) {
      // Keep what they typed so a usage slip is one edit away from valid.
      setDraft(restore);
      setHint(e instanceof Error ? e.message : String(e));
    } finally {
      setSending(false);
    }
  };

  const fillChip = (fill: string) => {
    setDraft(fill);
    setHint(null);
    inputRef.current?.focus();
  };

  return (
    <main className="flex min-h-0 min-w-0 flex-1 flex-col">
      <header className="flex items-center gap-2 border-b border-border px-6 py-4">
        <Hash className="size-4 text-muted-foreground" />
        <div>
          <h1 className="text-sm font-semibold leading-none">{space.title}</h1>
          <p className="mt-1 text-xs text-muted-foreground">#{space.slug}</p>
        </div>
      </header>

      <div className="min-h-0 flex-1 space-y-5 overflow-y-auto px-6 py-5">
        {messages.map((m) => (
          <div key={m.id} className="space-y-3">
            <TimelineMessage message={m} now={now} />
            {m.kind === "tool_request" && m.ref_id && pendingByRef.has(m.ref_id) && (
              <div className="pl-11">
                <ApprovalCard
                  approval={pendingByRef.get(m.ref_id)!}
                  onDecide={(status) => onDecide(m.ref_id!, status)}
                />
              </div>
            )}
          </div>
        ))}
        <div ref={endRef} />
      </div>

      <div className="border-t border-border px-6 py-4">
        <div className="mb-2 flex flex-wrap gap-2">
          {CHIPS.map((c) => (
            <Button
              key={c.label}
              size="sm"
              variant="outline"
              className="h-7 rounded-full px-3 text-xs"
              onClick={() => fillChip(c.fill)}
            >
              {c.label}
            </Button>
          ))}
        </div>
        {hint && (
          <pre className="mb-2 whitespace-pre-wrap rounded-[var(--radius-card)] border border-border bg-surface px-3 py-2 text-xs text-muted-foreground">
            {hint}
          </pre>
        )}
        <div className="flex items-end gap-2 rounded-[var(--radius-card)] border border-border bg-surface px-3 py-2">
          <textarea
            ref={inputRef}
            value={draft}
            onChange={(e) => setDraft(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter" && !e.shiftKey) {
                e.preventDefault();
                void send();
              }
            }}
            rows={1}
            placeholder={`Message #${space.slug}, or /task, /remind, /overview…`}
            className="max-h-40 min-h-9 flex-1 resize-none bg-transparent py-1.5 text-sm outline-none placeholder:text-muted-foreground"
          />
          <Button size="icon" disabled={sending || !draft.trim()} onClick={() => void send()}>
            <Send />
          </Button>
        </div>
      </div>
    </main>
  );
}
