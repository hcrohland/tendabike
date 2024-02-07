import {
  fmtDate,
  handleError,
  myfetch,
  updateSummary,
  usages,
} from "../lib/store";
import { filterValues, mapable, type Map } from "../lib/mapable";
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

  predecessors(services: Map<Service>) {
    let pred = filterValues(services, (s) => s.successor == this.id);
    let res = new Array();
    pred.forEach((s) => {
      res = res.concat(s.predecessors(services));
    });
    return pred.concat(res);
  }

  get_row(parts: Map<Part>, usages: Map<Usage>, services: Map<Service>) {
    let part = parts[this.part_id];
    let successor = this.get_successor(services);
    let next;
    let time: Date;
    if (!successor) {
      next = part.usage;
      time = new Date();
    } else {
      next = successor.usage;
      time = successor.time;
    }
    let days = Math.floor(
      (time.getTime() - this.time.getTime()) / (24 * 60 * 60 * 1000),
    );
    let usage = usages[next].sub(usages[this.usage]);
    return { service: this, days, usage };
  }

  fmtTime(s: Map<Service>) {
    let res = fmtDate(this.time);
    let successor = this.get_successor(s);
    if (successor) res = res + " - " + fmtDate(successor.time);
    return res;
  }
}

export const services = mapable("id", (s) => new Service(s));
