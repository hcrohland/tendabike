import { get_days, handleError, myfetch } from "../lib/store";
import { mapable, type Map, filterValues, by } from "../lib/mapable";
import { Service, services } from "../Service/service";
import { Part } from "../Part/part";
import {
  part_at_hook,
  type Attachment,
  attachment_for_part,
} from "../Attachment/attachment";
import type { Usage } from "../Usage/usage";

const is_set = (n: number | null) => n != null && n > 0;

export class Limits {
  /// Time until service
  days: number | null;
  /// Usage time
  time: number | null;
  /// Usage distance
  distance: number | null;
  /// Overall climbing
  climb: number | null;
  /// Overall descending
  descend: number | null;
  /// number of activities
  rides: number | null;

  constructor(data: any) {
    this.days = Number(data.days) || null;
    this.time = Number(data.time) || null;
    this.distance = Number(data.distance) || null;
    this.climb = Number(data.climb) || null;
    this.descend = Number(data.descend) || null;
    this.rides = Number(data.rides) || null;
  }

  static keys: (
    | "days"
    | "time"
    | "distance"
    | "climb"
    | "descend"
    | "rides"
  )[] = ["days", "time", "distance", "climb", "descend", "rides"];

  valid() {
    return (
      is_set(this.days) ||
      is_set(this.time) ||
      is_set(this.distance) ||
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
  part: number;
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

  services(part: Part, services: Map<Service>) {
    return filterValues(
      services,
      // @ts-ignore
      (s) => s.part_id == part.id && s.plans.includes(this.id),
    ).sort(by("time"));
  }

  getpart(parts: Map<Part>, attaches: Map<Attachment>) {
    return parts[part_at_hook(this.part, this.what, this.hook, attaches)];
  }

  due(part: Part, service: Service | undefined, usages: Map<Usage>) {
    let res = new Limits({});
    let time = service ? service.time : part.purchase;
    let usage = usages[part.usage];
    if (service) usage = usage.sub(usages[service.usage]);
    if (this.days) res.days = this.days - get_days(time);
    if (this.time) res.time = this.time - Math.floor(usage.time / 3600);
    if (this.distance)
      res.distance = this.distance - Math.floor(usage.distance / 1000);
    if (this.climb) res.climb = this.climb - usage.climb;
    if (this.descend) res.descend = this.descend - usage.descend;
    if (this.rides) res.rides = this.rides - usage.count;
    return res;
  }
}

export function plans_for_gear(
  part: number | undefined,
  plans: Map<ServicePlan>,
  atts: Map<Attachment>,
  time = new Date(),
) {
  let att = attachment_for_part(part, atts, time);
  return filterValues(
    plans,
    (s) =>
      s.part == part ||
      (s.hook == att?.hook && s.part == att?.gear && s.what == att?.what),
  );
}

export function plans_for_part(
  part: Part,
  time: Date,
  plans: Map<ServicePlan>,
  atts: Map<Attachment>,
) {
  if (part.isGear())
    return filterValues(plans, (p) => p.part == part.id && p.hook == null);
  else return plans_for_gear(part.id, plans, atts, time);
}
export const plans = mapable("id", (s) => new ServicePlan(s));