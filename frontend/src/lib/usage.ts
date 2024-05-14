import { Activity } from "./activity";
import { mapable } from "./mapable";

export class Usage {
  id: string;
  count: number;
  climb: number;
  descend: number;
  distance: number;
  time: number;
  duration: number;
  energy: number;

  constructor(data?: any) {
    if (data) {
      this.id = data.id;
      this.count = data.count;
      this.climb = data.climb;
      this.descend = data.descend;
      this.distance = data.distance;
      this.time = data.time;
      this.duration = data.duration;
      this.energy = data.energy;
    } else {
      this.id = "";
      this.count = 0;
      this.climb = 0;
      this.descend = 0;
      this.distance = 0;
      this.time = 0;
      this.duration = 0;
      this.energy = 0;
    }
  }

  add(a: Usage | Activity) {
    this.count += a.count || 1;
    this.climb += a.climb || 0;
    this.descend += a.descend || a.climb || 0;
    this.distance += a.distance || 0;
    this.time += a.time || a.duration || 0;
    this.duration += a.duration || a.time || 0;
    this.energy += a.energy || 0;
  }

  sub(rhs = new Usage()) {
    return new Usage({
      id: this.id,
      count: this.count - rhs.count,
      climb: this.climb - rhs.climb,
      descend: this.descend - rhs.descend,
      distance: this.distance - rhs.distance,
      time: this.time - rhs.time,
      duration: this.duration - rhs.duration,
      energy: this.energy - rhs.energy,
    });
  }
}

export const usages = mapable("id", (u) => new Usage(u));
