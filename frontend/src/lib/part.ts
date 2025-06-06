import { type Map, by, filterValues, mapable } from "./mapable";
import { handleError, myfetch } from "./store";
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
    this.purchase = new Date(data.purchase);
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
    return await myfetch("/api/part", "PUT", this)
      .then((data) => parts.updateMap([data]))
      .catch(handleError);
  }

  async dispose(date: Date) {
    this.disposed_at = date;
    await this.update();
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
