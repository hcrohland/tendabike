import { type Map, by, filterValues, fmtDate, handleError, myfetch, parts, types, updateSummary } from "./store";
export const maxDate = new Date("2999-12-31");

export class Usage {
  count?: number;
  climb?: number;
  descend?: number;
  distance?: number;
  time?: number;
  duration?: number;

  constructor(data?: any) {
    if (data) {
      this.count = data.count;
      this.climb = data.climb;
      this.descend = data.descend;
      this.distance = data.distance;
      this.time = data.time;
      this.duration = data.duration;
    } else {
      this.count = 0;
      this.climb = 0;
      this.descend = 0;
      this.distance = 0;
      this.time = 0;
      this.duration = 0;
    }
  }

  // fill () {
  //   if (! this.count) this.count = 0;
  //   if (! this.climb) this.climb = 0;
  //   if (! this.descend) this.descend = this.climb;
  //   if (! this.distance) this.distance = 0;
  //   if (! this.time) this.time = 0;
  //   if (! this.duration) this.duration = this.time;
  // }

  add(a: Usage) {
    this.count = (this.count || 0) + (a.count || 1);
    this.climb = (this.climb || 0) + (a.climb || 0);
    this.descend = (this.descend || 0) + (a.descend || a.climb || 0);
    this.distance = (this.distance || 0) + (a.distance || 0);
    this.time = (this.time || 0) + (a.time || a.duration || 0);
    this.duration = (this.duration || 0) + (a.duration || a.time || 0);
  }
}

export class Part extends Usage {
  id?: number;
  owner: number;
  what: number;
  name: string;
  vendor: string;
  model: string;
  purchase: Date;
  last_used: Date;
  disposed_at?: Date;

  constructor(data: any) {
    super(data);
    this.id = data.id;
    this.owner = data.owner;
    this.what = data.what;
    this.name = data.name;
    this.vendor = data.vendor;
    this.model = data.model;
    this.purchase = new Date(data.purchase);
    this.last_used = new Date(data.last_used);
    this.disposed_at = data.disposed_at ? new Date(data.disposed_at) : undefined;
  }

  async create() {
    return await myfetch('/part', 'POST', this)
      .then(data => { parts.updateMap([data]); return new Part(data) })
      .catch(handleError)
  }

  async update() {
    return await myfetch('/part', 'PUT', this)
      .then(data => parts.updateMap([data]))
      .catch(handleError)
  }

  type() {
    return types[this.what]
  }

  attachments(atts: Map<Attachment>) {
    return filterValues(atts, (a) => a.part_id == this.id).sort(by("attached"))
  }

  isGear() {
    return this.type().main == this.what
  }

}

export class Attachment extends Usage {
  part_id: number;
  attached: Date;
  gear: number;
  hook: number;
  detached: Date;
  what: number;
  name: string;
  idx: string;
  constructor(data: any) {
    super(data);
    this.part_id = data.part_id;
    this.attached = new Date(data.attached);
    this.gear = data.gear;
    this.hook = data.hook;
    this.detached = new Date(data.detached);
    this.what = data.what;
    this.name = data.name;
    this.idx = this.part_id + "/" + this.attached.getTime()
  }
  fmtTime() {
    let res = fmtDate(this.attached);
    if (this.detached < maxDate)
      res = res + " - " + fmtDate(this.detached)
    return res
  }
  isAttached(time?: Date | string | number) {
    if (!time) time = new Date()
    return this.attached <= time && time < this.detached
  }
  isEmpty() {
    return this.attached.getTime() >= this.detached.getTime()
  }
  activities(acts: Map<Activity>) {
    return filterValues(acts, (a) => a.gear == this.gear && this.isAttached(a.start))
  }
}

export class Type {
  id: number;
  name: string;
  main: number;
  hooks: Array<number>;
  order: number;
  group?: string;
  prefix: string;
  acts: ActType[];

  // export let types: { [key: number]: Type };
  constructor(t: any) {
    this.id = t.id;
    this.name = t.name;
    this.main = t.main;
    this.hooks = t.hooks;
    this.order = t.order;

    this.prefix = this.name.split(' ').reverse()[1] || '';  // The first word iff there were two (hack!)
    this.acts = [];
  }

  activities(acts: Map<Activity>) {
    return filterValues(acts, (a) =>
      this.acts.some((t) => t.id == a.what),
    ).sort(by("start"));
  }

  parts(parts: Map<Part>) {
    return filterValues(parts, (p) => p.what == this.id).sort(
      by("last_used"));
  }
}

export type User = {
  id: number,
  firstname: string,
  name: string,
  is_admin: boolean
}

export class Activity extends Usage {
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

  constructor(data: any) {
    super(data);
    this.id = data.id;
    this.user_id = data.user_id;
    this.what = data.what;
    this.name = data.name;
    this.start = new Date(data.start);
    this.gear = data.gear;
  }
}

export type ActType = {
  id: number,
  name: string,
  gear_type: number
}

export class AttEvent {
  part_id: number;
  time: Date;
  gear: number;
  hook: number;
  constructor(part: number | undefined, time: Date, gear: number | undefined, hook: number) {
    if (gear == undefined || part == undefined) {
      console.error("part or gear not defined: ", part, gear);
      throw ("part or gear not defined")
    }
    this.part_id = part;
    this.time = time;
    this.gear = gear;
    this.hook = hook;
  }

  async attach() {
    return await myfetch('/part/attach', 'POST', this)
      .then(updateSummary)
      .catch(handleError)
  }

  async detach() {
    return await myfetch('/part/detach', 'POST', this)
      .then(updateSummary)
      .catch(handleError)
  }
}
