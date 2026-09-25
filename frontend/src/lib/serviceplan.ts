import {
  att_at_hook,
  attachment_for_part,
  attachments,
  part_at_hook,
  type Attachment,
} from "./attachment";
import { by, type Map, mapableState, stateValues } from "./mapable.svelte";
import { Part, parts } from "./part";
import { Service, services } from "./service";
import { get_days, handleError, myfetch } from "./store";
import { Type, types } from "./types";
import type { Usage } from "./usage";
import { m } from "../../paraglide/messages";

export type limit_keys =
  "days" | "hours" | "km" | "climb" | "descend" | "rides" | "kJ";

const is_set = (n: number | null) => n != null && n > 0;

export class Limits {
  /// Time until service
  days: number | null;
  /// Usage time
  hours: number | null;
  /// Usage distance
  km: number | null;
  /// Overall climbing
  climb: number | null;
  /// Overall descending
  descend: number | null;
  /// number of activities
  rides: number | null;
  /// energy in kilJoule
  kJ: number | null;

  constructor(data: any) {
    this.days = parseInt(data.days) || null;
    this.hours = parseInt(data.hours) || null;
    this.km = parseInt(data.km) || null;
    this.climb = parseInt(data.climb) || null;
    this.descend = parseInt(data.descend) || null;
    this.rides = parseInt(data.rides) || null;
    this.kJ = parseInt(data.kJ) || null;
  }

  static keys: limit_keys[] = [
    "days",
    "rides",
    "hours",
    "km",
    "climb",
    "descend",
    "kJ",
  ];

  static valid(l: any) {
    return (
      is_set(l.days) ||
      is_set(l.hours) ||
      is_set(l.km) ||
      is_set(l.climb) ||
      is_set(l.descend) ||
      is_set(l.rides) ||
      is_set(l.kJ)
    );
  }

  to_object() {
    return { ...this };
  }

  set_from_object(a: any) {
    Object.assign(this, a);
  }
}

export class ServicePlan extends Limits {
  id?: string;
  /// the gear or part involved
  /// if hook is None the plan is for a specific part
  /// if it's Some(hook) it is a generic plan for that hook
  part: number | null;
  /// This is only really used for generic plans
  /// for a specific part it is set to the PartType of the part
  what: number;
  /// where it is attached
  hook: number | null;
  name: string;
  constructor(data: any) {
    super(data);
    this.id = data.id || "00000000-0000-0000-0000-000000000000";
    this.part = data.part;
    this.what = data.what;
    this.hook = data.hook || null;

    this.name = data.name || "";
  }

  async create() {
    return await myfetch("/api/plan", "POST", this)
      .then((data) => plans.updateMap([data]))
      .catch(handleError);
  }

  async update() {
    return await myfetch("/api/plan", "PUT", this)
      .then((data) => plans.updateMap([data]))
      .catch(handleError);
  }

  async delete() {
    await myfetch("/api/plan/" + this.id, "DELETE")
      .then((data) => services.updateMap(data))
      .catch(handleError);
    plans.deleteItem(this.id);
  }

  valid() {
    return Limits.valid(this) && this.name.length > 0 && this.what != undefined;
  }

  services(part: Part | null, services: Map<Service>) {
    return stateValues(services)
      .filter((s) => s.part_id == part?.id && s.plans.includes(this.id!))
      .sort(by("time"));
  }
}

/** Resolve the physical part for a plan's gear: the part attached at the
 * plan's hook on the gear, falling back to the gear part itself.
 */
function part_for_plan(plan: ServicePlan, gear?: number): Part | null {
  let part = gear ? gear : plan.part;
  return part
    ? parts[part_at_hook(part, plan.what, plan.hook, attachments)]
    : null;
}

/** Sort order for plan lists: type, then hook, then part, then id; nulls
 * sort first via a -1 sentinel; ids compare as strings.
 */
export function planCmp(a: ServicePlan, b: ServicePlan): number {
  if (a.what != b.what) return a.what < b.what ? -1 : 1;
  let ah = a.hook ?? -1;
  let bh = b.hook ?? -1;
  if (ah != bh) return ah < bh ? -1 : 1;
  let ap = a.part ?? -1;
  let bp = b.part ?? -1;
  if (ap != bp) return ap < bp ? -1 : 1;
  return a.id! < b.id! ? -1 : a.id! > b.id! ? 1 : 0;
}

/**
 * Determines the physical parts a service plan is associated with.
 * If linked to a specific part directly, it returns that one immediately.
 * For generic plans, it resolves to all active matching components
 * that do not already have a dedicated maintenance plan assigned
 * (matching either a gear-level or a component-level specific plan).
 */
function gears_of_plan(
  plan: ServicePlan,
  parts: Map<Part>,
  plans: ServicePlan[],
  atts: Map<Attachment>,
): Part[] {
  if (plan.part) return [parts[plan.part]];

  let main = types[plan.what].main;
  return stateValues(parts).filter((p) => {
    if (p.disposed_at != null || main != p.what) return false;
    let att = att_at_hook(p.id!, plan.what, plan.hook, atts);
    return !plans.some(
      (r) =>
        (r.part == p.id && r.hook == plan.hook && r.what == plan.what) ||
        (att != null && r.part == att.part_id),
    );
  });
}

/** Remaining limits after the part's usage since its latest service.
 * A limit of zero or unset means the key is inactive; hours and km use
 * floor semantics; the usage delta is taken from the latest service.
 */
function due_for(
  plan: ServicePlan,
  part: Part | null,
  service: Service | undefined,
  usages: Map<Usage>,
): Limits {
  let res = new Limits({});
  if (part == null || part.what != plan.what) return res;
  let time = service ? service.time : part.purchase;
  let usage = usages[part.usage];
  if (service) usage = usage.sub(usages[service.usage]);
  if (plan.days) res.days = plan.days - get_days(time);
  if (plan.hours) res.hours = plan.hours - Math.floor(usage.time / 3600);
  if (plan.km) res.km = plan.km - Math.floor(usage.distance / 1000);
  if (plan.climb) res.climb = plan.climb - usage.climb;
  if (plan.descend) res.descend = plan.descend - usage.descend;
  if (plan.rides) res.rides = plan.rides - usage.count;
  if (plan.kJ) res.kJ = plan.kJ - usage.energy;
  return res;
}

/** Per-limit severity verdict. */
type severity = "ok" | "warn" | "alert";

/**
 * Per-limit severity: "alert" when the remaining is negative, "warn"
 * when it is within 5% of the limit, else "ok". The badge and chip
 * render from this verdict; the cross-limit scan folds it below.
 */
function severity_for(due: number, plan: number): severity {
  if (due < 0) return "alert";
  if (due < plan * 0.05) return "warn";
  return "ok";
}

/** Severity of a plan for its part: "alert" when any due value is
 * negative, else "warn" when any due value is within 5% of its limit.
 * The scan runs in the fixed key order, skipping zero-remaining limits,
 * with overdue beating the band.
 */
function alert_for(
  plan: ServicePlan,
  part: Part,
  service: Service | undefined,
  usages: Map<Usage>,
): severity | "" {
  let res: severity | "" = "";
  let due = due_for(plan, part, service, usages);
  for (const key of ServicePlan.keys) {
    if (!due[key]) continue;
    let s = severity_for(due[key]!, plan[key]!);
    if (s == "alert") res = "alert";
    else if (s == "warn" && res == "") res = "warn";
  }
  return res;
}

/*** find plans for this part only */
function plans_for_this_part(
  part_id: number | undefined,
  plans: Map<ServicePlan>,
) {
  return stateValues(plans).filter((p) => p.part == part_id && p.hook == null);
}

/** return plans for this part.
 *
 * If there is a specific plan for this part, the generic plans do not apply
 */
function plans_for_attachee(plans: Map<ServicePlan>, att: Attachment) {
  // find plans for this part and generic plan for gear
  let res = stateValues(plans).filter(
    (p) =>
      p.part == att.part_id ||
      (p.part == att.gear && p.hook == att.hook && p.what == att.what),
  );
  if (res.length != 0) return res;

  // find generic plans for this type/hook
  return stateValues(plans)
    .filter(
      (p) =>
        p.part == null &&
        p.hook == att.hook &&
        p.what == att.what &&
        // only if the is none already for this part already
        !res.some((r) => r.hook == p.hook && r.what == p.what),
    )
    .map((p) => new ServicePlan({ ...p, part: att.part_id }));
}

/** Plans for a part at a given time or now, in record order (unsorted);
 * the caller sorts with planCmp.
 */
function plans_for_part_at(
  part: number | undefined,
  plans: Map<ServicePlan>,
  atts: Map<Attachment>,
  time: Date = new Date(),
): ServicePlan[] {
  let att = attachment_for_part(part, atts, time);
  return att
    ? plans_for_attachee(plans, att)
    : plans_for_this_part(part, plans);
}

function plans_at_hook(
  atts: Map<Attachment>,
  plans: Map<ServicePlan>,
  part: Part,
  type: Type,
  hook: number,
) {
  let att = att_at_hook(part.id!, type.id, hook, atts);
  if (att) return plans_for_attachee(plans, att);

  let res = stateValues(plans).filter(
    (p) => p.part == part.id && p.what == type.id && p.hook == hook,
  );
  if (res.length > 0) return res;

  return stateValues(plans)
    .filter((p) => p.part == null && p.what == type.id && p.hook == hook)
    .map((p) => new ServicePlan({ ...p, part: part.id }));
}

function plans_for_subtype(
  atts: Map<Attachment>,
  plans: Map<ServicePlan>,
  part: Part,
  type: Type,
) {
  return type.hooks.reduce((res, hook) => {
    return res.concat(plans_at_hook(atts, plans, part, type, hook));
  }, [] as ServicePlan[]);
}

/** Plans for a part and the parts it assembles through its type's subtype
 * hooks, in walk order (unsorted); the caller sorts with planCmp.
 */
function plans_for_assembly(
  part: Part,
  plans: Map<ServicePlan>,
  atts: Map<Attachment>,
): ServicePlan[] {
  return types[part.what]
    .subtypes()
    .reduce(
      (list, type) => list.concat(plans_for_subtype(atts, plans, part, type)),
      plans_for_part_at(part.id, plans, atts),
    );
}

export function localizeLimitKey(key: limit_keys): string {
  // SAFETY: the paraglide module exposes one function per compiled message
  // key; an unknown key is checked below and falls back to the raw key.
  const fn = (m as unknown as Record<string, () => string>)[`limit_${key}`];
  return typeof fn === "function" ? fn() : key;
}

/*
 * Doors: the narrow public surface of the plan rule (issue #323).
 * Signatures carry only entity values and time; the rule reads the module's
 * state objects in the function bodies. Dependencies register at the
 * enclosing reactive call site.
 */

/**
 * The plan module's per-limit due value: the remaining `due`, the limit
 * `plan`, and the severity verdict the module computes for it (see
 * severity_for). The badge and chip render from it; the part-card status
 * module sequenced after this ticket exposes the same value. One-export
 * extension of the locked #323 surface (issue #345).
 */
export class Due {
  due: number;
  plan: number;
  severity: severity;
  constructor(due: number, plan: number, severity: severity) {
    this.due = due;
    this.plan = plan;
    this.severity = severity;
  }
}

/**
 * Resolve the physical part for a plan's gear: the part attached at the
 * plan's hook on the gear, falling back to the gear part itself.
 */
export function partForPlanGear(plan: ServicePlan, gear?: number): Part | null {
  return part_for_plan(plan, gear);
}

/**
 * The physical parts a service plan is associated with: its specific part,
 * or the active, uncovered parts of the type for a generic plan.
 */
export function gearsForPlan(
  plan: ServicePlan,
  parts: Map<Part>,
  attachments: Map<Attachment>,
  plans: Map<ServicePlan>,
): Part[] {
  return gears_of_plan(plan, parts, stateValues(plans), attachments);
}

/**
 * Band counts over the plans' parts: how many sit in the warn band and
 * how many are overdue.
 */
export function alertCounts(
  plans: ServicePlan[],
  parts: Map<Part>,
  services: Map<Service>,
  usages: Map<Usage>,
  attachments: Map<Attachment>,
): { warn: number; alert: number } {
  let res = { warn: 0, alert: 0 };
  plans.forEach((plan) => {
    gears_of_plan(plan, parts, plans, attachments).forEach((gear) => {
      let part = part_for_plan(plan, gear.id);
      if (part != null) {
        let serviceList = plan.services(part, services);
        let alert = alert_for(plan, part, serviceList.at(0), usages);
        if (alert == "warn") res.warn++;
        else if (alert == "alert") res.alert++;
      }
    });
  });
  return res;
}

/**
 * Plans for a part at a pinned time or now, in record order (unsorted);
 * the caller sorts with planCmp.
 */
export function plansForPart(
  part: number | undefined,
  plans: Map<ServicePlan>,
  attachments: Map<Attachment>,
  time: Date = new Date(),
): ServicePlan[] {
  return plans_for_part_at(part, plans, attachments, time);
}

/**
 * Plans for a part and the parts it assembles through its type's subtype
 * hooks, in walk order (unsorted); the caller sorts with planCmp.
 */
export function plansForAssembly(
  part: Part,
  plans: Map<ServicePlan>,
  attachments: Map<Attachment>,
): ServicePlan[] {
  return plans_for_assembly(part, plans, attachments);
}

/**
 * Whether a plan is a template (not bound to a specific part): true for a
 * generic plan, or when the plan is not (or no longer) in the map.
 */
export function isTemplate(
  plan: ServicePlan,
  plans: Map<ServicePlan>,
): boolean {
  return typeof (plan.id && plans[plan.id]?.part) !== "number";
}

/**
 * Nearest remaining limit per key across the part's plan list, as the
 * module's per-limit NextDue value (remaining, limit, severity verdict).
 */
export function duesForPlans(
  part: Part | null,
  plans: ServicePlan[],
  services: Map<Service>,
  usages: Map<Usage>,
): Partial<Record<limit_keys, Due>> {
  const result: Partial<Record<limit_keys, Due>> = {};
  for (const plan of plans) {
    const serviceList = plan.services(part, services);
    const due = due_for(plan, part, serviceList.at(0), usages);
    for (const key of Limits.keys) {
      const p = plan[key] as number | null;
      const d = due[key] as number | null;
      if (p == null || d == null) continue;
      if (result[key] == null || d < result[key]!.due) {
        result[key] = new Due(d, p, severity_for(d, p));
      }
    }
  }
  return result;
}

export const plans = mapableState("id", (s) => new ServicePlan(s));
