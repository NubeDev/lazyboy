import type { ApprovalStatus, MessageKind, RunStatus, TaskState } from "@/rpc/types";

// The store has no author column on messages; the timeline shows who is
// speaking by mapping the message kind. Human turns are the local user;
// every agent/tool turn is the goose worker.
export function authorFor(kind: MessageKind): string {
  switch (kind) {
    case "human":
      return "You";
    case "ingress":
    case "system":
      return "System";
    default:
      return "Goose";
  }
}

type Tone = "neutral" | "accent" | "success" | "warning" | "danger";

export const runStatusTone: Record<RunStatus, Tone> = {
  queued: "neutral",
  running: "accent",
  waiting_approval: "warning",
  succeeded: "success",
  failed: "danger",
  cancelled: "neutral",
};

export const taskStateTone: Record<TaskState, Tone> = {
  open: "neutral",
  running: "accent",
  blocked_on_approval: "warning",
  done: "success",
  cancelled: "neutral",
};

export const approvalTone: Record<ApprovalStatus, Tone> = {
  pending: "warning",
  approved: "success",
  denied: "danger",
};

export function humanize(s: string): string {
  return s.replace(/_/g, " ");
}

export function relativeTime(iso: string, now: number): string {
  const diff = now - Date.parse(iso);
  if (diff < 60_000) return "just now";
  const mins = Math.round(diff / 60_000);
  if (mins < 60) return `${mins}m ago`;
  const hrs = Math.round(mins / 60);
  if (hrs < 24) return `${hrs}h ago`;
  return `${Math.round(hrs / 24)}d ago`;
}

export function absoluteTime(iso: string): string {
  return new Date(iso).toLocaleString(undefined, {
    month: "short",
    day: "numeric",
    hour: "numeric",
    minute: "2-digit",
  });
}

// A pending reminder whose due time has passed needs the danger tone; a
// fired or dismissed reminder is settled regardless of due_at.
export function isOverdue(dueIso: string, now: number): boolean {
  return Date.parse(dueIso) < now;
}

// A datetime-local input yields a naive local value ("2026-06-13T14:30");
// the wire expects RFC3339, so anchor it to the browser's zone.
export function localInputToIso(value: string): string {
  return new Date(value).toISOString();
}
