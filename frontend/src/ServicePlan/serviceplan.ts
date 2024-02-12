import { handleError, myfetch, updateSummary } from "../lib/store";
import { mapable, type Map } from "../lib/mapable";
import { services } from "../Service/service";

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
  count: number | null;

  constructor(data: any) {
    this.days = Number(data.days) || null;
    this.time = Number(data.time) || null;
    this.distance = Number(data.distance) || null;
    this.climb = Number(data.climb) || null;
    this.descend = Number(data.descend) || null;
    this.count = Number(data.count) || null;
  }

  static keys: (
    | "days"
    | "time"
    | "distance"
    | "climb"
    | "descend"
    | "count"
  )[] = ["days", "time", "distance", "climb", "descend", "count"];

  check() {
    return (
      is_set(this.days) ||
      is_set(this.time) ||
      is_set(this.distance) ||
      is_set(this.climb) ||
      is_set(this.descend) ||
      is_set(this.count)
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
    this.id = "00000000-0000-0000-0000-000000000000";
    this.part = data.part;
    this.what = data.what || data.part.what;
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
}

export const plans = mapable("id", (s) => new ServicePlan(s));
