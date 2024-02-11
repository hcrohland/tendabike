import { type Map, by, filterValues, mapable } from "../lib/mapable";
import { handleError, myfetch, types } from "../lib/store";
import { Attachment } from "../Attachment/attachment";

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
}

export const parts = mapable("id", (p) => new Part(p));
