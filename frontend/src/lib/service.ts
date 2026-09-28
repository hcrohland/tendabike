import { handleError, myfetch } from "./store";
import { mapableState, stateValues } from "./mapable.svelte";
import { usages } from "./usage";
import { updateSummary } from "./user";

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

  async repeat() {
    return await myfetch("/api/service/redo", "POST", this)
      .then(updateSummary)
      .catch(handleError);
  }

  get_successor(): Service | null {
    if (!this.successor) return null;

    // this might happen when the lists get updated
    if (!services[this.successor]) {
      // console.error("Successor of ", this, "does not exist");
      return null;
    }

    return services[this.successor];
  }

  history(depth: number): {
    depth: number;
    service: Service | undefined;
    successor: Service;
  }[] {
    let preds = stateValues(services).filter((s) => s.successor == this.id);
    if (preds.length > 0) {
      let res = new Array();
      preds.forEach((service, i) => {
        // the early ones have the higher depth!
        let d = depth + preds.length - (i + 1);
        res.push({ depth: d, service, successor: this });
        res = res.concat(service.history(d));
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
}

export const services = mapableState("id", (s) => new Service(s));
