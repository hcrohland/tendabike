/**
 * The notification state module: the unified client model over three
 * lifecycles.
 *
 *   ephemeral  — client-only toasts; dismiss deletes; never in history/badge.
 *   transient  — server-stored; in the panel while present, in the badge;
 *                dismiss = `read` API (the frame's tombstone removes it).
 *   persistent — server-stored; in the panel, in the badge while unread;
 *                dismiss = `read` API (stays, shown as read).
 *
 * Server notifications merge through `updateSummary` (the notifications map);
 * frames are idempotent by id. Ephemeral toasts are created locally by the
 * `error()` door.
 */
import { mapableState, stateValues } from "./mapable.svelte";
import { getUser } from "./user.svelte";
import { myfetch, handleError } from "./utils";

/// The wire shape of a server notification.
export type Notification = {
  id: string;
  recipient: number;
  lifecycle: "transient" | "persistent";
  kind:
    | "sub_request"
    | "sub_response"
    | "sub_cancelled"
    | "part_registered"
    | "part_unregistered";
  subject: unknown;
  message?: string;
  state: "new" | "seen";
  created_at: string;
};

/// A client-only ephemeral toast.
export type Toast = {
  id: string;
  kind: "error";
  message: string;
  created_at: number;
};

/// The server notification collection, keyed by id. Fed by `updateSummary`.
export const notifications = mapableState<Notification>("id");

/// Ephemeral toasts: a separate `$state` array, never in the mapable
/// collection, never in the badge or history.
let toasts = $state<Toast[]>([]);

/// Whether the notification panel (drop-down) is open.
let panelOpen = $state(false);

export function isPanelOpen() {
  return panelOpen;
}
export function setPanelOpen(v: boolean) {
  panelOpen = v;
}

let toastSeq = 0;

/**
 * Doors — derived views and actions, exposed as functions.
 */

/// The ephemeral toasts, newest first.
export function toastsDoor(): Toast[] {
  return [...toasts].reverse();
}

/// The server notifications (transient + persistent), newest first.
/// Filtered to the current user's notifications only (recipient guard).
export function history(): Notification[] {
  const uid = getUser()?.id;
  return stateValues(notifications)
    .filter((n) => n.recipient === uid)
    .sort(
      (a, b) =>
        new Date(b.created_at).getTime() - new Date(a.created_at).getTime(),
    );
}

/// The badge count: unread persistent + all transient (transient are always
/// "unread" until the tombstone removes them). Filtered to the current
/// user's notifications only (recipient guard).
export function unread(): number {
  const uid = getUser()?.id;
  return stateValues(notifications).filter(
    (n) =>
      n.recipient === uid &&
      (n.lifecycle === "transient" ||
        (n.lifecycle === "persistent" && n.state === "new")),
  ).length;
}

/// Dismiss a notification: the lifecycle decides the outcome.
///   ephemeral  → deleted locally
///   transient  → `read` API (tombstone removes it)
///   persistent → `read` API (stays as seen)
export function dismiss(id: string) {
  // Ephemeral toast?
  const idx = toasts.findIndex((t) => t.id === id);
  if (idx !== -1) {
    toasts = toasts.filter((t) => t.id !== id);
    return;
  }
  // Server notification: call the read API.
  const n = notifications[id];
  if (!n) return;
  apiRead(id);
}

/// Mark all server notifications as read.
export function readAll() {
  apiReadAll();
}

/// The ephemeral door: create a local error toast.
export function error(message: string) {
  toastSeq++;
  toasts = [
    ...toasts,
    { id: "toast_" + toastSeq, kind: "error", message, created_at: Date.now() },
  ];
}

/// Clear all ephemeral toasts.
export function clearToasts() {
  toasts = [];
}

/// Read a single notification (lifecycle-aware: persistent → mark seen,
/// transient → delete). The change rides the stream frame.
async function apiRead(id: string) {
  await myfetch("/api/notification/" + id + "/read", "POST").catch(handleError);
}

/// Read all of the user's notifications (lifecycle-aware for each).
async function apiReadAll() {
  await myfetch("/api/notification/read-all", "POST").catch(handleError);
}
