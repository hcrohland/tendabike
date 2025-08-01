import type { Activity } from "./activity";
import { filterValues, mapable, type Map } from "./mapable";
import { fmtRange, maxDate } from "./store";

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
    return fmtRange(this.attached, this.detached);
  }

  isAttached(time?: Date | string | number) {
    if (!time) time = new Date();
    else time = new Date(time);

    return this.attached <= time && time < this.detached;
  }

  isDetached() {
    return this.detached < maxDate;
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

/// find attachment for part at a specific hook right now
export function att_at_hook(
  gear: number,
  what: number,
  hook: number | null,
  atts: Map<Attachment>,
) {
  return filterValues(
    atts,
    (att) =>
      att.gear == gear &&
      att.what == what &&
      att.hook == hook &&
      att.isAttached(),
  ).pop();
}

/// find part id for part at a specific hook right now
/// if there is no part at that hook, return parameter part
export function part_at_hook(
  gear: number,
  what: number,
  hook: number | null,
  atts: Map<Attachment>,
) {
  let att = att_at_hook(gear, what, hook, atts);
  return att ? att.part_id : gear;
}

/***
  return the attachment for part at time or undefined if it is not attached
*/
export function attachment_for_part(
  part: number | undefined,
  atts: Map<Attachment>,
  time: Date,
) {
  return filterValues(
    atts,
    (att) => att.part_id == part && att.attached <= time && att.detached > time,
  ).pop();
}

export function attachees_for_gear(
  gear: number | undefined,
  atts: Map<Attachment>,
  time = new Date(),
) {
  return filterValues(
    atts,
    (att) => att.gear == gear && att.attached <= time && att.detached > time,
  );
}

export const attachments = mapable(
  "idx",
  (a) => new Attachment(a),
  (a) => a.isEmpty(),
);
