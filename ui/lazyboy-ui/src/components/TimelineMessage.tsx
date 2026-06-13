import { Bot, User, Terminal, FileOutput, Inbox, Settings2 } from "lucide-react";
import type { Message, MessageKind } from "@/rpc/types";
import { authorFor, relativeTime } from "@/lib/labels";
import { cn } from "@/lib/utils";

const kindIcon: Record<MessageKind, typeof Bot> = {
  human: User,
  agent: Bot,
  system: Settings2,
  tool_request: Terminal,
  tool_result: FileOutput,
  artifact_ref: FileOutput,
  decision_ref: Settings2,
  ingress: Inbox,
};

interface Props {
  message: Message;
  now: number;
}

export function TimelineMessage({ message, now }: Props) {
  const Icon = kindIcon[message.kind];
  const isAgent = message.kind === "agent";
  const isTool = message.kind === "tool_request" || message.kind === "tool_result";

  return (
    <div className="flex gap-3">
      <div
        className={cn(
          "mt-0.5 flex size-8 shrink-0 items-center justify-center rounded-lg",
          isAgent ? "bg-accent/20 text-accent" : "bg-muted text-muted-foreground",
        )}
      >
        <Icon className="size-4" />
      </div>
      <div className="min-w-0 flex-1">
        <div className="flex items-baseline gap-2">
          <span className="text-sm font-medium">{authorFor(message.kind)}</span>
          <span className="text-xs text-muted-foreground">{relativeTime(message.ts, now)}</span>
        </div>
        <div
          className={cn(
            "mt-0.5 text-sm leading-relaxed",
            isTool ? "font-mono text-xs text-muted-foreground" : "text-foreground",
          )}
        >
          {message.body}
        </div>
      </div>
    </div>
  );
}
