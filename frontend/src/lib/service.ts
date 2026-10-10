import { get_days, handleError, myfetch } from "./store";
import { mapableState, stateValues } from "./mapable.svelte";
import { usages, Usage } from "./usage";
import type { Part } from "./part";

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

  /// The writes answer 204 (201 for creates): the change rides the stream
  /// frame, so the handlers only await the write.
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
    }).catch(handleError);
  }

  async update() {
    return await myfetch("/api/service", "PUT", this).catch(handleError);
  }

  async delete() {
    await myfetch("/api/service/" + this.id, "DELETE").catch(handleError);
  }

  async repeat() {
    return await myfetch("/api/service/redo", "POST", this).catch(handleError);
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

  /**
   * The usage window between a service and its successor: the usage
   * accumulated in the window and the days that passed. Without a service
   * the window starts at the part's purchase with an empty usage; without a
   * successor it ends now with the part's current usage. Reads the module's
   * usages state object in the body; dependencies register at the enclosing
   * reactive call site. Pure: mutates nothing.
   */
  static period(
    service: Service | null | undefined,
    part: Part,
    successor: Service | null | undefined,
  ): { usage: Usage; days: number } {
    let start = service ? service.time : part.purchase;
    let startUsage = service ? usages[service.usage] : new Usage();
    let end = successor ? successor.time : new Date();
    let endUsage = successor ? usages[successor.usage] : usages[part.usage];
    return { usage: endUsage.sub(startUsage), days: get_days(start, end) };
  }
}

export const services = mapableState("id", (s) => new Service(s));
