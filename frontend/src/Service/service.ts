import { fmtDate, handleError, myfetch, updateSummary } from "../lib/store";
import { filterValues, mapable, type Map } from "../lib/mapable";
import { Part } from "../Part/part";
import { usages, Usage } from "../Usage/usage";

export class Service {
  id?: string;
  part_id: number;
  /// when it was serviced
  time: Date;
  // this is not used any more
  redone: Date;
  name: string;
  notes: string;
  // we do not accept thos values from the client!
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

    // this might happen when the lists get updated
    if (!s[this.successor]) {
      // console.error("Successor of ", this, "does not exist");
      return null;
    }

    return s[this.successor];
  }

  history(
    depth: number,
    parts: Map<Part>,
    usages: Map<Usage>,
    services: Map<Service>,
  ) {
    let pred = filterValues(services, (s) => s.successor == this.id);
    if (pred.length > 0) {
      let res = new Array();
      pred.forEach((s, i) => {
        // the early ones have the higher depth!
        let d = depth + pred.length - (i + 1);
        res.push(s.get_row(d, parts, usages, services));
        res = res.concat(s.history(d, parts, usages, services));
      });
      return res;
    } else {
      // build a service entry for the part when it was new
      let first = new Service({
        id: "pred" + this.id,
        name: "┗━",
        notes: "",
        part_id: this.part_id,
        successor: this.id,
      });
      return new Array(first.get_row(depth, parts, usages, services));
    }
  }

  get_row(
    depth: number,
    parts: Map<Part>,
    usages: Map<Usage>,
    services: Map<Service>,
  ) {
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
    // this.usage is undefined for the period without a service
    // this period starts at time part.purchase and has an empty usage
    if (!this.usage) this.time = part.purchase;
    let usage = this.usage
      ? usages[next].sub(usages[this.usage])
      : usages[next];

    // How many days passed
    let days = Math.floor(
      (time.getTime() - this.time.getTime()) / (24 * 60 * 60 * 1000),
    );
    return { depth, service: this, days, usage };
  }

  fmtTime(s: Map<Service>) {
    let res = fmtDate(this.time);
    let successor = this.get_successor(s);
    if (successor) res = res + " - " + fmtDate(successor.time);
    return res;
  }
}

export const services = mapable("id", (s) => new Service(s));
