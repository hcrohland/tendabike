import {
  fmtDate,
  handleError,
  myfetch,
  services,
  updateSummary,
  usages,
} from "../lib/store";
import { maxDate } from "../lib/types";

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

  constructor(data: any) {
    this.id = data.id;
    this.part_id = data.part_id;
    this.time = data.time ? new Date(data.time) : new Date();
    this.redone = data.redone ? new Date(data.redone) : new Date();
    this.name = data.name || "";
    this.notes = data.notes || "";
    this.usage = data.usage;
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
    await myfetch("/service/" + this.id, "DELETE").catch(handleError);
    services.deleteItem(this.id);
    usages.deleteItem(this.usage);
  }

  async redo() {
    return await myfetch("/service/redo", "POST", this)
      .then(updateSummary)
      .catch(handleError);
  }

  fmtTime() {
    let res = fmtDate(this.time);
    if (this.redone < maxDate) res = res + " - " + fmtDate(this.redone);
    return res;
  }
}
