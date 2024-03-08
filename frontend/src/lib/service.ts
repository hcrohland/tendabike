import {
  fmtDate,
  get_days,
  handleError,
  myfetch,
  updateSummary,
} from "./store";
import { filterValues, mapable, type Map } from "./mapable";
import { Part } from "./part";
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
  successor: string | null;
  plans: string[];

  constructor(data: any) {
    this.id = data.id;
    this.part_id = data.part_id;
    this.time = data.time ? new Date(data.time) : new Date();
    this.redone = data.redone ? new Date(data.redone) : new Date();
    this.name = data.name || "";
    this.notes = data.notes || "";
    this.usage = data.usage;
    this.successor = data.successor || null;
    this.plans = data.plans || [];
  }

  static async create(
    part_id: number,
    time: Date,
    name: string,
    notes: string,
    plans: string[],
  ) {
    return await myfetch("/api/service", "POST", {
      part_id,
      time,
      name,
      notes,
      plans,
    })
      .then(updateSummary)
      .catch(handleError);
  }

  async update() {
    return await myfetch("/api/service", "PUT", this)
      .then(updateSummary)
      .catch(handleError);
  }

  async delete() {
    await myfetch("/api/service/" + this.id, "DELETE")
      .then(updateSummary)
      .catch(handleError);
    services.deleteItem(this.id);
    usages.deleteItem(this.usage);
  }

  async redo() {
    return await myfetch("/api/service/redo", "POST", this)
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
    services: Map<Service>,
  ): { depth: number; service: Service | undefined; successor: Service }[] {
    let preds = filterValues(services, (s) => s.successor == this.id);
    if (preds.length > 0) {
      let res = new Array();
      preds.forEach((service, i) => {
        // the early ones have the higher depth!
        let d = depth + preds.length - (i + 1);
        res.push({ depth: d, service, successor: this });
        res = res.concat(service.history(d, services));
      });
      return res;
    } else {
      return new Array({
        depth: depth - 1,
        service: undefined,
        successor: this,
      });
    }
  }

  get_row(
    depth: number,
    part: Part,
    usages: Map<Usage>,
    successor: Service | null,
  ) {
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
    let days = get_days(this.time, time);
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
