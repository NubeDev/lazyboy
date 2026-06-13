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
  onSend: (prompt: string) => Promise<void>;
  onDecide: (approvalId: string, status: ApprovalStatus) => Promise<void>;
}

export function SpaceTimeline({ space, messages, pending, now, onSend, onDecide }: Props) {
  const [draft, setDraft] = useState("");
  const [sending, setSending] = useState(false);
  const endRef = useRef<HTMLDivElement>(null);
  const pendingByRef = new Map(pending.map((a) => [a.id, a]));

  useEffect(() => {
    endRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages.length]);

  const send = async () => {
    const text = draft.trim();
    if (!text || sending) return;
    setSending(true);
    setDraft("");
    try {
      await onSend(text);
    } finally {
      setSending(false);
    }
  };

  return (
    <main className="flex min-w-0 flex-1 flex-col">
      <header className="flex items-center gap-2 border-b border-border px-6 py-4">
        <Hash className="size-4 text-muted-foreground" />
        <div>
          <h1 className="text-sm font-semibold leading-none">{space.title}</h1>
          <p className="mt-1 text-xs text-muted-foreground">#{space.slug}</p>
        </div>
      </header>

      <div className="flex-1 space-y-5 overflow-y-auto px-6 py-5">
        {messages.map((m) => (
          <div key={m.id} className="space-y-3">
            <TimelineMessage message={m} now={now} />
            {m.kind === "tool_request" && m.refId && pendingByRef.has(m.refId) && (
              <div className="pl-11">
                <ApprovalCard
                  approval={pendingByRef.get(m.refId)!}
                  onDecide={(status) => onDecide(m.refId!, status)}
                />
              </div>
            )}
          </div>
        ))}
        <div ref={endRef} />
      </div>

      <div className="border-t border-border px-6 py-4">
        <div className="flex items-end gap-2 rounded-[var(--radius-card)] border border-border bg-surface px-3 py-2">
          <textarea
            value={draft}
            onChange={(e) => setDraft(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter" && !e.shiftKey) {
                e.preventDefault();
                void send();
              }
            }}
            rows={1}
            placeholder={`Drop an idea into #${space.slug}…`}
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
