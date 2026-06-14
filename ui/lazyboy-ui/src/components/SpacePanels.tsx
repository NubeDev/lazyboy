import { useState, type ComponentType } from "react";
import {
  ListTodo,
  GitCommitVertical,
  BellRing,
  CalendarDays,
  Users,
} from "lucide-react";
import type {
  AgentRun,
  CalendarEvent,
  Decision,
  Group,
  Membership,
  Reminder,
  Task,
  UpsertCalendarBody,
} from "@/rpc/types";
import { cn } from "@/lib/utils";
import { TaskPanelContent } from "./TaskPanel";
import { DecisionsPanel } from "./DecisionsPanel";
import { RemindersPanel } from "./RemindersPanel";
import { CalendarPanel } from "./CalendarPanel";
import { MembersPanel } from "./MembersPanel";

type PrincipalKind = "user" | "group";

interface Props {
  tasks: Task[];
  runs: AgentRun[];
  decisions: Decision[];
  reminders: Reminder[];
  calendar: CalendarEvent[];
  groups: Group[];
  members: Membership[];
  now: number;
  onRecordDecision: (summary: string) => Promise<void>;
  onDismissReminder: (id: string) => Promise<void>;
  onCreateReminder: (body: string, dueAtIso: string) => Promise<void>;
  onCreateCalendar: (body: UpsertCalendarBody) => Promise<void>;
  onGrantMembership: (
    principalKind: PrincipalKind,
    principalId: string,
    role: string,
  ) => Promise<void>;
  onCreateGroup: (name: string) => Promise<void>;
}

type TabId = "tasks" | "decisions" | "reminders" | "calendar" | "members";

const TABS: { id: TabId; label: string; icon: ComponentType<{ className?: string }> }[] = [
  { id: "tasks", label: "Tasks", icon: ListTodo },
  { id: "decisions", label: "Decisions", icon: GitCommitVertical },
  { id: "reminders", label: "Reminders", icon: BellRing },
  { id: "calendar", label: "Calendar", icon: CalendarDays },
  { id: "members", label: "Members", icon: Users },
];

export function SpacePanels(props: Props) {
  const [tab, setTab] = useState<TabId>("tasks");

  return (
    <aside className="flex w-80 shrink-0 flex-col overflow-hidden border-l border-border bg-surface">
      <nav className="flex border-b border-border">
        {TABS.map(({ id, label, icon: Icon }) => (
          <button
            key={id}
            title={label}
            onClick={() => setTab(id)}
            className={cn(
              "flex flex-1 items-center justify-center py-3 text-muted-foreground transition-colors hover:bg-muted",
              tab === id && "border-b-2 border-accent text-foreground",
            )}
          >
            <Icon className="size-4" />
          </button>
        ))}
      </nav>

      <div className="flex-1 overflow-y-auto px-4 py-5">
        {tab === "tasks" && (
          <div className="flex flex-col gap-6">
            <TaskPanelContent tasks={props.tasks} runs={props.runs} />
          </div>
        )}
        {tab === "decisions" && (
          <DecisionsPanel
            decisions={props.decisions}
            now={props.now}
            onRecord={props.onRecordDecision}
          />
        )}
        {tab === "reminders" && (
          <RemindersPanel
            reminders={props.reminders}
            now={props.now}
            onDismiss={props.onDismissReminder}
            onCreate={props.onCreateReminder}
          />
        )}
        {tab === "calendar" && (
          <CalendarPanel calendar={props.calendar} onCreate={props.onCreateCalendar} />
        )}
        {tab === "members" && (
          <MembersPanel
            members={props.members}
            groups={props.groups}
            onGrant={props.onGrantMembership}
            onCreateGroup={props.onCreateGroup}
          />
        )}
      </div>
    </aside>
  );
}
