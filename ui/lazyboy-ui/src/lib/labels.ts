import type { ApprovalStatus, RunStatus, TaskState } from "@/rpc/types";

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
