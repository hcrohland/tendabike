import { Service, services } from "./service";
import { activities, Activity } from "./activity";
import { Usage, usages } from "./usage";
import { parts, type Part } from "./part";
import { partNotes, type PartNote } from "./partnote";
import { Attachment, attachments } from "./attachment";
import { plans, type ServicePlan } from "./serviceplan";
import { Shop, shops } from "./shop";
import { myfetch } from "./store";
import { mapableState, type StateMap } from "./mapable.svelte";
import { getUser, setUser } from "./user.svelte";

export { getUser, setUser };

export async function initData() {
  let u = await myfetch("/api/user");
  if (u) {
    setUser(u);
  } else {
    return;
  }
  return refresh();
}

export async function refresh(shop?: number) {
  let query = shop ? "?shop=" + shop : "";
  await myfetch("/api/user/summary" + query).then(setSummary);
}

export type UserPublic = {
  id: number;
  firstname: string;
  name: string;
  avatar: string | undefined;
};

export type OnboardingStatus =
  "pending" | "completed" | "initial_sync_postponed";

export type User = {
  id: number;
  firstname: string;
  name: string;
  avatar: string | undefined;
  is_admin: boolean;
  onboarding_status: OnboardingStatus;
};

/// The wire shape of a Summary: a uniform object per collection, with
/// stringified id keys and `null` for tombstones (an entity that no longer
/// exists). The attachment keys are the client's `Attachment.idx`
/// (`part_id + "/" + attached ms`) — the backend's `idx()` produces the same
/// format. Exported so the stream client can type the frames it merges.
export type Summary = {
  parts: Record<string, Part | null>;
  part_notes: Record<string, PartNote | null>;
  attachments: Record<string, Attachment | null>;
  activities: Record<string, Activity | null>;
  usages: Record<string, Usage | null>;
  services: Record<string, Service | null>;
  plans: Record<string, ServicePlan | null>;
  shops: Record<string, Shop | null>;
  users: Record<string, UserPublic | null>;
};

export const users = mapableState<UserPublic>("id");

/// One registry row: a Summary field paired with the state collection it
/// feeds.
type SummaryRow<K extends keyof Summary> = {
  key: K;
  collection: StateMap<NonNullable<Summary[K][string]>>;
};

/// Any one row of the registry.
type AnySummaryRow = { [K in keyof Summary]: SummaryRow<K> }[keyof Summary];

/**
 * The sync registry (issue #412): one row per Summary field, pairing the
 * payload field with the state collection it feeds. Both merge lanes fold
 * over it, so the table is the single answer to "which collections does a
 * Summary feed?" The row type pins each key to its own collection, so a
 * field removed from Summary, or a collection drifting from its field, is
 * a compile error here. The table is module-private by design: the two
 * merge functions are the interface.
 *
 * The rows are built on each call, not captured at module load: this module
 * and the entity modules import each other (entity methods call back into
 * `updateSummary`), so an array literal evaluated here would read a
 * collection binding while that module is still mid-import — undefined for
 * whichever of them the import graph placed behind this one.
 */
function summaryRows(): AnySummaryRow[] {
  return [
    { key: "usages", collection: usages },
    { key: "parts", collection: parts },
    { key: "part_notes", collection: partNotes },
    { key: "attachments", collection: attachments },
    { key: "activities", collection: activities },
    { key: "services", collection: services },
    { key: "plans", collection: plans },
    { key: "shops", collection: shops },
    { key: "users", collection: users },
  ];
}

/**
 * The merge ops of a summary collection, over the union of all Summary
 * field value types. The nine value types are heterogeneous, so no single
 * strict parameter can name them all; the union keeps each row's own
 * collection satisfying the fold below.
 */
type SummaryFieldOps = {
  setMap(arr: Summary[keyof Summary][string][]): void;
  updateMap(arr: Summary[keyof Summary][string][]): void;
  deleteItem(id: string | number | undefined): void;
};

/// Hydration lane: replaces every collection the Summary feeds, unwrapping
/// the object values (tombstones carry no entity, so they are dropped —
/// `setMap` replaces the whole record anyway).
export function setSummary(data: Summary) {
  for (const row of summaryRows()) {
    const values = Object.values(data[row.key] as Record<string, any>).filter(
      (v) => v !== null,
    );
    (row.collection as SummaryFieldOps).setMap(values);
  }
}

/// Update lane: merges every collection the Summary feeds, per id — a
/// `null` value (tombstone) deletes the row it names, any other value is
/// upserted. An absent payload falls back to a full refresh.
///
/// Attachment keying: the wire key is the client's `Attachment.idx`
/// (`part_id + "/" + attached ms`, which the deep-link scheme depends on), so
/// a `null` attachment names the client row its `deleteItem` removes. The
/// backend's `AttachmentDetail::idx` produces the same format; any live
/// attachment, an empty window included, is upserted under it.
export function updateSummary(data?: Summary) {
  if (!data) {
    refresh();
    return;
  }
  for (const row of summaryRows()) {
    const ops = row.collection as SummaryFieldOps;
    const upserts: any[] = [];
    for (const [id, value] of Object.entries(
      data[row.key] as Record<string, any>,
    )) {
      if (value === null) ops.deleteItem(id);
      else upserts.push(value);
    }
    if (upserts.length > 0) ops.updateMap(upserts);
  }
}
