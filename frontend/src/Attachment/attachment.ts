import type { Activity } from "../Activity/activity";
import { type Map, filterValues, mapable } from "../lib/mapable";
import { fmtDate, handleError, myfetch, updateSummary } from "../lib/store";
import { maxDate } from "../lib/types";

export class Attachment {
  part_id: number;
  attached: Date;
  gear: number;
  hook: number;
  detached: Date;
  what: number;
  name: string;
  idx: string;
  usage: string;
  constructor(data: any) {
    this.part_id = data.part_id;
    this.attached = new Date(data.attached);
    this.gear = data.gear;
    this.hook = data.hook;
    this.detached = new Date(data.detached);
    this.what = data.what;
    this.name = data.name;
    this.idx = this.part_id + "/" + this.attached.getTime();
    this.usage = data.usage;
  }
  fmtTime() {
    let res = fmtDate(this.attached);
    if (this.detached < maxDate) res = res + " - " + fmtDate(this.detached);
    return res;
  }
  isAttached(time?: Date | string | number) {
    if (!time) time = new Date();
    else time = new Date(time);

    return this.attached <= time && time < this.detached;
  }
  isEmpty() {
    return this.attached.getTime() >= this.detached.getTime();
  }
  activities(acts: Map<Activity>) {
    return filterValues(
      acts,
      (a) => a.gear == this.gear && this.isAttached(a.start),
    );
  }
}

export class AttEvent {
  part_id: number;
  time: Date;
  gear: number;
  hook: number;
  constructor(
    part: number | undefined,
    time: Date,
    gear: number | undefined,
    hook: number,
  ) {
    if (gear == undefined || part == undefined) {
      console.error("part or gear not defined: ", part, gear);
      throw "part or gear not defined";
    }
    this.part_id = part;
    this.time = time;
    this.gear = gear;
    this.hook = hook;
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

/// find part id for part at a specific hook right now
/// if there is no part at that hook, return parameter part
export function part_at_hook(
  gear: number,
  what: number,
  hook: number | null,
  atts: Map<Attachment>,
) {
  let att = filterValues(
    atts,
    (att) =>
      att.gear == gear &&
      att.what == what &&
      att.hook == hook &&
      att.isAttached(),
  ).pop();
  return att ? att.part_id : gear;
}

export function attachment_for_part(
  part: number | undefined,
  atts: Map<Attachment>,
) {
  let now = new Date();
  return filterValues(
    atts,
    (att) => att.part_id == part && att.attached <= now && att.detached > now,
  ).pop();
}

export const attachments = mapable(
  "idx",
  (a) => new Attachment(a),
  (a) => a.isEmpty(),
);
