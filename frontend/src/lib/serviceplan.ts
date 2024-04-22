import { get_days, handleError, myfetch } from "./store";
import { mapable, type Map, filterValues, by } from "./mapable";
import { Service, services } from "./service";
import { Part } from "./part";
import {
  part_at_hook,
  type Attachment,
  attachment_for_part,
  attachees_for_gear,
} from "./attachment";
import type { Usage } from "./usage";
import { types } from "./types";

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

  constructor(data: any) {
    this.days = parseInt(data.days) || null;
    this.hours = parseInt(data.hours) || null;
    this.km = parseInt(data.km) || null;
    this.climb = parseInt(data.climb) || null;
    this.descend = parseInt(data.descend) || null;
    this.rides = parseInt(data.rides) || null;
  }

  static keys: ("days" | "hours" | "km" | "climb" | "descend" | "rides")[] = [
    "days",
    "hours",
    "km",
    "climb",
    "descend",
    "rides",
  ];

  valid() {
    return (
      is_set(this.days) ||
      is_set(this.hours) ||
      is_set(this.km) ||
      is_set(this.climb) ||
      is_set(this.descend) ||
      is_set(this.rides)
    );
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
    return super.valid() && this.name.length > 0 && this.what != undefined;
  }

  services(part: Part | null, services: Map<Service>) {
    return filterValues(
      services,
      (s) => s.part_id == part?.id && s.plans.includes(this.id!),
    ).sort(by("time"));
  }

  getpart(parts: Map<Part>, attaches: Map<Attachment>) {
    return this.part
      ? parts[part_at_hook(this.part, this.what, this.hook, attaches)]
      : null;
  }

  due(part: Part | null, service: Service | undefined, usages: Map<Usage>) {
    let res = new Limits({});
    if (part == null) return res;
    let time = service ? service.time : part.purchase;
    let usage = usages[part.usage];
    if (service) usage = usage.sub(usages[service.usage]);
    if (this.days) res.days = this.days - get_days(time);
    if (this.hours) res.hours = this.hours - Math.floor(usage.time / 3600);
    if (this.km) res.km = this.km - Math.floor(usage.distance / 1000);
    if (this.climb) res.climb = this.climb - usage.climb;
    if (this.descend) res.descend = this.descend - usage.descend;
    if (this.rides) res.rides = this.rides - usage.count;
    return res;
  }

  alert(part: Part, service: Service | undefined, usages: Map<Usage>) {
    let res = "";
    let due = this.due(part, service, usages);
    for (const key of ServicePlan.keys) {
      if (due[key]) {
        if (due[key]! < 0) res = "alert";
        if (res == "" && due[key]! < this[key]! * 0.05) res = "warn";
      }
    }
    return res;
  }

  no_template(plans: Map<ServicePlan>) {
    // Warning: The plan might vanish from Map during deletion!
    return this.id && plans[this.id]?.part;
  }

  partLink(parts: Map<Part>) {
    return this.part ? parts[this.part].partLink() : "";
  }

  gears(parts: Map<Part>, plans: Map<ServicePlan>) {
    if (this.part) return [];

    let main = types[this.what].main;
    return filterValues(
      parts,
      (p) =>
        p.disposed_at == null &&
        main == p.what &&
        !Object.values(plans).some(
          (r) => r.part == p.id && r.hook == this.hook,
        ),
    );
  }
}

function plans_for_this_part(
  part_id: number | undefined,
  plans: Map<ServicePlan>,
) {
  return filterValues(plans, (p) => p.part == part_id && p.hook == null);
}

function plans_for_attachee(
  plans: Map<ServicePlan>,
  att: Attachment | undefined,
) {
  let res = filterValues(
    plans,
    (p) =>
      p.part == att?.part_id ||
      (p.part == att?.gear && p.hook == att?.hook && p.what == att?.what),
  );
  filterValues(
    plans,
    (p) => p.part == null && p.hook == att?.hook && p.what == att?.what,
  ).forEach((p) => {
    if (!res.some((r) => r.hook == p.hook && r.what == p.what))
      res.push(new ServicePlan({ ...p, part: att?.gear }));
  });
  return res;
}

export function plans_for_part(
  plans: Map<ServicePlan>,
  atts: Map<Attachment>,
  part: number | undefined,
  time: Date = new Date(),
) {
  let att = attachment_for_part(part, atts, time);
  return att
    ? plans_for_attachee(plans, att)
    : plans_for_this_part(part, plans);
}

export function plans_for_part_and_attachees(
  atts: Map<Attachment>,
  plans: Map<ServicePlan>,
  part: number | undefined,
) {
  let attachees = attachees_for_gear(part, atts);
  return attachees.reduce(
    (list, att) => list.concat(plans_for_attachee(plans, att)),
    plans_for_part(plans, atts, part),
  );
}

export function alerts_for_plans(
  plans: ServicePlan[],
  parts: Map<Part>,
  services: Map<Service>,
  usages: Map<Usage>,
  attachments: Map<Attachment>,
) {
  let res = { warn: 0, alert: 0 };
  plans.forEach((plan) => {
    let part = plan.getpart(parts, attachments);
    if (part != null) {
      let serviceList = plan.services(part, services);
      let alert = plan.alert(part, serviceList.at(0), usages);

      if (alert == "warn") res.warn++;
      else if (alert == "alert") res.alert++;
    }
  });
  return res;
}

export const plans = mapable("id", (s) => new ServicePlan(s));
