import { Part } from "./part";
import { mapable, type Map } from "./mapable";
import { handleError, myfetch, updateSummary } from "./store";

export class Activity {
  id: number;
  /// The athlete
  user_id: number;
  /// The activity type
  what: number;
  /// This name of the activity.
  name: string;
  /// Start time
  start: Date;
  /// Which gear did she use?
  gear?: number;
  count: number = 1;
  climb?: number;
  descend?: number;
  distance?: number;
  time?: number;
  duration?: number;

  constructor(data: any) {
    this.id = data.id;
    this.user_id = data.user_id;
    this.what = data.what;
    this.name = data.name;
    this.start = new Date(data.start);
    this.gear = number_or_undefined(data.gear);
    this.climb = number_or_undefined(data.climb);
    this.descend = number_or_undefined(data.descend);
    this.distance = number_or_undefined(data.distance);
    this.time = number_or_undefined(data.time);
    this.duration = number_or_undefined(data.duration);
  }

  async update() {
    let a = new Activity(this);
    return await myfetch("/api/activ/" + a.id, "PUT", a)
      .then((data) => updateSummary(data))
      .catch(handleError);
  }

  gearLink(parts: Map<Part>) {
    if (this.gear && parts[this.gear]) {
      return parts[this.gear].partLink();
    } else {
      return "-";
    }
  }

  gearName(parts: Map<Part>) {
    return this.gear && parts[this.gear] ? parts[this.gear].name : "-";
  }
}

function number_or_undefined(e: any) {
  let res = parseInt(e);
  return Number.isNaN(res) ? undefined : res;
}
export const activities = mapable("id", (a) => new Activity(a));
