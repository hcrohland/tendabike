import {
  fmtDate,
  handleError,
  myfetch,
  services,
  updateSummary,
  usages,
} from "../lib/store";
import { type Map } from "../lib/mapable";
import type { Part, Usage } from "../lib/types";

export class Service {
  id?: string;
  part_id: number;
  /// when it was serviced
  time: Date;
  /// when there was a new service
  redone: Date;
  // we do not accept theses values from the client!
  name: string;
  notes: string;
  usage: string;
  successor?: string;

  constructor(data: any) {
    this.id = data.id;
    this.part_id = data.part_id;
    this.time = data.time ? new Date(data.time) : new Date();
    this.redone = data.redone ? new Date(data.redone) : new Date();
    this.name = data.name || "";
    this.notes = data.notes || "";
    this.usage = data.usage;
    this.successor = data.successor;
  }

  static async create(
    part_id: number,
    time: Date,
    name: string,
    notes: string,
  ) {
    return await myfetch("/service", "POST", {
      part_id,
      time,
      name,
      notes,
    })
      .then(updateSummary)
      .catch(handleError);
  }

  async update() {
    return await myfetch("/service", "PUT", this)
      .then(updateSummary)
      .catch(handleError);
  }

  async delete() {
    await myfetch("/service/" + this.id, "DELETE")
      .then(updateSummary)
      .catch(handleError);
    services.deleteItem(this.id);
    usages.deleteItem(this.usage);
  }

  async redo() {
    return await myfetch("/service/redo", "POST", this)
      .then(updateSummary)
      .catch(handleError);
  }

  get_successor(s: Map<Service>) {
    if (!this.successor) return null;

    if (!s[this.successor]) {
      console.error("Successor of ", this, "does not exist");
      return null;
    }

    return s[this.successor];
  }

  get_use(part: Part, usages: Map<Usage>, services: Map<Service>) {
    let successor = this.get_successor(services);
    let next;
    if (!successor) next = part.usage;
    else {
      next = successor.usage;
    }
    return usages[next].sub(usages[this.usage]);
  }

  fmtTime(s: Map<Service>) {
    let res = fmtDate(this.time);
    let successor = this.get_successor(s);
    if (successor) res = res + " - " + fmtDate(successor.time);
    return res;
  }
}
