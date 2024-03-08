import { Part } from "../lib/part";
import { mapable, type Map } from "../lib/mapable";

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
  count: number;
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
    this.gear = data.gear;
    this.climb = data.climb;
    this.descend = data.descend;
    this.distance = data.distance;
    this.time = data.time;
    this.duration = data.duration;
    this.count = 1;
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

export const activities = mapable("id", (a) => new Activity(a));
