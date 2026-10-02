import { Activity, activities as allActivities } from "./activity";
import { parts, type Part } from "./part";
import { mapableState, stateValues } from "./mapable.svelte";
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

  activities(): Activity[] {
    return stateValues(allActivities).filter(
      (a) => a.gear == this.gear && this.isAttached(a.start),
    );
  }
}

/// find attachment for part at a specific hook right now
export function att_at_hook(gear: number, what: number, hook: number | null) {
  return stateValues(attachments)
    .filter(
      (att) =>
        att.gear == gear &&
        att.what == what &&
        att.hook == hook &&
        att.isAttached(),
    )
    .pop();
}

/// find part id for part at a specific hook right now
/// if there is no part at that hook, return parameter part
export function part_at_hook(
  gear: number,
  what: number,
  hook: number | null,
): number {
  let att = att_at_hook(gear, what, hook);
  return att ? att.part_id : gear;
}

/***
  return the attachment for part at time or undefined if it is not attached
*/
export function attachment_for_part(part: number | undefined, time: Date) {
  return stateValues(attachments)
    .filter(
      (att) =>
        att.part_id == part && att.attached <= time && att.detached > time,
    )
    .pop();
}

/***
  return the part this part is attached to at time — the top-level gear of the
  assembly via `attachment_for_part` and the `gear` lookup — or undefined if it
  is not attached
*/
export function mounted_on(
  part: number | undefined,
  time: Date,
): Part | undefined {
  const att = attachment_for_part(part, time);
  if (!att) return;
  return parts[att.gear];
}

/***
  the latest attach or detach boundary strictly before `t`, over the part's
  rows (if a part is given) and the slot's rows (gear + hook + part type);
  undefined when there is no earlier boundary
*/
export function prev_attach_time(
  t: Date,
  part: number | undefined,
  gear: number | undefined,
  hook: number | undefined,
  what: number | undefined,
): Date | undefined {
  let last: Date | undefined;
  for (const a of stateValues(attachments)) {
    if (
      a.part_id == part ||
      (a.gear == gear && a.hook == hook && a.what == what)
    ) {
      if (a.attached < t && (!last || a.attached > last)) last = a.attached;
      if (a.detached < t && (!last || a.detached > last)) last = a.detached;
    }
  }
  return last;
}

/***
  the default attach date for a part installed at the slot: now if the slot
  has an attachment row, else the gear's purchase date
*/
export function default_attach_date(
  gear: number,
  hook: number | undefined,
  what: number | undefined,
): Date {
  const hasRow = stateValues(attachments).some(
    (a) => a.gear == gear && a.hook == hook && a.what == what,
  );
  if (hasRow) return new Date();
  return parts[gear]?.purchase ?? new Date();
}

export const attachments = mapableState(
  "idx",
  (a) => new Attachment(a),
  (a) => a.isEmpty(),
);
