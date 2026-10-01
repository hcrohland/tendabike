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

type Summary = {
  parts: Part[];
  part_notes: PartNote[];
  attachments: Attachment[];
  activities: Activity[];
  usages: Usage[];
  services: Service[];
  plans: ServicePlan[];
  shops: Shop[];
  users: UserPublic[];
};

export const users = mapableState<UserPublic>("id");

/// One registry row: a Summary field paired with the state collection it
/// feeds.
type SummaryRow<K extends keyof Summary> = {
  key: K;
  collection: StateMap<Summary[K][number]>;
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
 * field types. The nine field types are heterogeneous, so no single strict
 * parameter can name them all; method syntax keeps the parameters
 * bivariant, letting each row's own collection satisfy the fold below.
 */
type SummaryFieldOps = {
  setMap(arr: Summary[keyof Summary]): void;
  updateMap(arr: Summary[keyof Summary]): void;
};

/// Hydration lane: replaces every collection the Summary feeds.
export function setSummary(data: Summary) {
  for (const row of summaryRows()) {
    (row.collection as SummaryFieldOps).setMap(data[row.key]);
  }
}

/// Update lane: merges every collection the Summary feeds — an upsert by id
/// that never removes rows, so a partial Summary can never empty a
/// collection. An absent payload falls back to a full refresh.
export function updateSummary(data?: Summary) {
  if (!data) {
    refresh();
    return;
  }
  for (const row of summaryRows()) {
    (row.collection as SummaryFieldOps).updateMap(data[row.key]);
  }
}
