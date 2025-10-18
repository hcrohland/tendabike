import { type Map, by, filterValues, mapable } from "./mapable";
import { handleError, myfetch, updateSummary } from "./store";
import { Attachment } from "./attachment";
import { Type, types } from "./types";
import { Activity } from "./activity";

export class Part {
  id?: number;
  owner: number;
  what: number;
  name: string;
  vendor: string;
  model: string;
  purchase: Date;
  last_used: Date;
  disposed_at?: Date;
  usage: string;

  constructor(data: any) {
    this.id = data.id;
    this.owner = data.owner;
    this.what = data.what;
    this.name = data.name || "";
    this.vendor = data.vendor || "";
    this.model = data.model || "";
    this.purchase = data.purchase ? new Date(data.purchase) : new Date();
    this.last_used = new Date(data.last_used);
    this.disposed_at = data.disposed_at
      ? new Date(data.disposed_at)
      : undefined;
    this.usage = data.usage;
  }

  async create() {
    return await myfetch("/api/part", "POST", this)
      .then((data) => {
        parts.updateMap([data]);
        return new Part(data);
      })
      .catch(handleError);
  }

  async update() {
    return await myfetch("/api/part/" + this.id, "PUT", this)
      .then((data) => parts.updateMap([data]))
      .catch(handleError);
  }
  async detach(date: Date, all: boolean) {
    await new AttEvent(this.id!, date, all, 0, 0).detach();
  }

  async attach(date: Date, all: boolean, gear: number, hook: number) {
    await new AttEvent(this.id!, date, all, gear, hook).attach();
  }

  async dispose(date: Date, all: boolean) {
    await new DisposeEvent(this.id!, all, date).dispose();
  }

  async recover(all: boolean) {
    await new DisposeEvent(this.id!, all).recover();
  }

  type() {
    return types[this.what];
  }

  attachments(atts: Map<Attachment>) {
    return filterValues(atts, (a) => a.part_id == this.id).sort(by("attached"));
  }

  isGear() {
    return this.type().main == this.what;
  }

  partLink() {
    return (
      '<a href="/#/part/' +
      this.id +
      '" style="text-decoration1:none" class="text-reset">' +
      this.name +
      "</a>"
    );
  }

  /// the time when the first activity or attachment for this part started
  firstEvent(acts: Map<Activity>, atts: Map<Attachment>) {
    return this.isGear()
      ? filterValues(acts, (a) => a.gear == this.id)
          .sort(by("start"))
          .at(-1)?.start
      : this.attachments(atts).at(-1)?.attached;
  }
}

export function allGear(parts: Map<Part>, category: Type) {
  return filterValues(
    parts,
    (p) => p.what == category.main && p.disposed_at != null,
  );
}

export const parts = mapable("id", (p) => new Part(p));

class AttEvent {
  part_id: number;
  time: Date;
  gear: number;
  hook: number;
  all: boolean;
  constructor(
    part: number,
    time: Date,
    all: boolean = true,
    gear: number = 0,
    hook: number = 0,
  ) {
    if (part == undefined) {
      console.error("part not defined: ", part);
      throw "part not defined";
    }
    this.part_id = part;
    this.time = time;
    this.gear = gear;
    this.hook = hook;
    this.all = all;
  }

  async attach() {
    return await myfetch("/api/part/attach", "POST", this)
      .then(updateSummary)
      .catch(handleError);
  }

  async detach() {
    return await myfetch("/api/part/detach", "POST", this)
      .then(updateSummary)
      .catch(handleError);
  }
}

class DisposeEvent {
  part_id: number;
  time: Date;
  all: boolean;
  constructor(part: number, all: boolean, time: Date = new Date()) {
    this.part_id = part;
    this.time = time;
    this.all = all;
  }
  async dispose() {
    return await myfetch("/api/part/dispose", "POST", this)
      .then(updateSummary)
      .catch(handleError);
  }

  async recover() {
    return await myfetch("/api/part/recover", "POST", this)
      .then(updateSummary)
      .catch(handleError);
  }
}
