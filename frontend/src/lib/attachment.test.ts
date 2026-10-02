import { describe, expect, it, beforeEach } from "vitest";
import {
  Attachment,
  attachments,
  att_at_hook,
  attachment_for_part,
  mounted_on,
  part_at_hook,
} from "./attachment";
import { Part, parts } from "./part";
import { Activity, activities } from "./activity";
import { maxDate } from "./store";
import { stateValues, type Map } from "./mapable.svelte";

function att(overrides: Partial<any> = {}): Attachment {
  return new Attachment({
    part_id: 1,
    attached: "2023-01-01T00:00:00Z",
    gear: 100,
    hook: 1,
    detached: maxDate.getTime(),
    what: 10,
    name: "part",
    usage: "u1",
    ...overrides,
  });
}

function attMap(...atts: Attachment[]): Map<Attachment> {
  return Object.fromEntries(atts.map((a) => [a.idx, a])) as Map<Attachment>;
}

// Test-only helper, moved out of attachment.ts (issue #387); it stays
// pure over its local map.
function attacheesForGear(
  gear: number | undefined,
  atts: Map<Attachment>,
  time = new Date(),
) {
  return stateValues(atts).filter(
    (att) => att.gear == gear && att.attached <= time && att.detached > time,
  );
}

describe("Attachment.idx", () => {
  it("is built from part_id and the attached time", () => {
    const a = att({ part_id: 7, attached: "2023-01-01T00:00:00Z" });
    expect(a.idx).toBe(7 + "/" + new Date("2023-01-01T00:00:00Z").getTime());
  });
});

describe("Attachment.isAttached", () => {
  const window = () =>
    att({ attached: "2020-01-01T00:00:00Z", detached: "2022-12-31T00:00:00Z" });

  it("is true within the window", () => {
    expect(window().isAttached(new Date("2021-06-01T00:00:00Z"))).toBe(true);
  });

  it("is true at the exact attached boundary", () => {
    expect(window().isAttached(new Date("2020-01-01T00:00:00Z"))).toBe(true);
  });

  it("is false before the attached time", () => {
    expect(window().isAttached(new Date("2019-06-01T00:00:00Z"))).toBe(false);
  });

  it("is false at the exact detached boundary (exclusive)", () => {
    expect(window().isAttached(new Date("2022-12-31T00:00:00Z"))).toBe(false);
  });

  it("is false after the detached time", () => {
    expect(window().isAttached(new Date("2023-06-01T00:00:00Z"))).toBe(false);
  });

  it("defaults to the current time", () => {
    expect(att().isAttached()).toBe(true);
  });
});

describe("Attachment.isDetached", () => {
  it("is false for a still-attached attachment", () => {
    expect(att().isDetached()).toBe(false);
  });

  it("is true once detached in the past", () => {
    expect(att({ detached: "2022-12-31T00:00:00Z" }).isDetached()).toBe(true);
  });
});

describe("Attachment.isEmpty", () => {
  it("is false for a real attachment", () => {
    expect(
      att({
        attached: "2020-01-01T00:00:00Z",
        detached: "2022-12-31T00:00:00Z",
      }).isEmpty(),
    ).toBe(false);
  });

  it("is true when attached equals detached", () => {
    expect(
      att({
        attached: "2020-01-01T00:00:00Z",
        detached: "2020-01-01T00:00:00Z",
      }).isEmpty(),
    ).toBe(true);
  });

  it("is true when attached is after detached", () => {
    expect(
      att({
        attached: "2022-01-01T00:00:00Z",
        detached: "2020-01-01T00:00:00Z",
      }).isEmpty(),
    ).toBe(true);
  });
});

describe("Attachment.fmtTime", () => {
  it("shows a single date while still attached", () => {
    const a = att({ attached: "2023-01-01T00:00:00Z" });
    expect(a.fmtTime()).toBe(
      new Date("2023-01-01T00:00:00Z").toLocaleDateString(navigator.language),
    );
  });

  it("shows a range once detached", () => {
    const a = att({
      attached: "2020-01-01T00:00:00Z",
      detached: "2022-12-31T00:00:00Z",
    });
    const s = new Date("2020-01-01T00:00:00Z").toLocaleDateString(
      navigator.language,
    );
    const e = new Date("2022-12-31T00:00:00Z").toLocaleDateString(
      navigator.language,
    );
    expect(a.fmtTime()).toBe(s + " - " + e);
  });
});

// The doors read the module state objects in their bodies (#387); seed the
// world and reset it between tests.
function reset() {
  activities.setMap([]);
  attachments.setMap([]);
}

describe("Attachment.activities", () => {
  beforeEach(reset);

  it("returns gear-matching activities within the attachment window", () => {
    const a = att({
      attached: "2020-01-01T00:00:00Z",
      detached: "2022-12-31T00:00:00Z",
      gear: 100,
    });
    activities.setMap([
      new Activity({ id: 1, gear: 100, start: "2021-06-01T00:00:00Z" }),
      new Activity({ id: 2, gear: 100, start: "2023-06-01T00:00:00Z" }),
      new Activity({ id: 3, gear: 999, start: "2021-06-01T00:00:00Z" }),
    ]);
    expect(a.activities().map((x) => x.id)).toEqual([1]);
  });
});

describe("att_at_hook", () => {
  beforeEach(reset);

  it("returns the currently-attached attachment at a hook", () => {
    const current = att({ part_id: 1, attached: "2023-01-01T00:00:00Z" });
    const old = att({
      part_id: 2,
      attached: "2020-01-01T00:00:00Z",
      detached: "2022-12-31T00:00:00Z",
    });
    attachments.setMap([current, old]);
    // The collection's prep function rebuilds the attachment, so the door
    // returns a value-equal attachment, not the seeded instance.
    expect(att_at_hook(100, 10, 1)).toEqual(current);
  });

  it("returns undefined when nothing is currently attached", () => {
    const old = att({
      part_id: 2,
      attached: "2020-01-01T00:00:00Z",
      detached: "2022-12-31T00:00:00Z",
    });
    attachments.setMap([old]);
    expect(att_at_hook(100, 10, 1)).toBeUndefined();
  });
});

describe("part_at_hook", () => {
  beforeEach(reset);

  it("returns the part id of the attached attachment", () => {
    const current = att({ part_id: 7, attached: "2023-01-01T00:00:00Z" });
    attachments.setMap([current]);
    expect(part_at_hook(100, 10, 1)).toBe(7);
  });

  it("falls back to the gear when nothing matches", () => {
    const current = att({ part_id: 7, attached: "2023-01-01T00:00:00Z" });
    attachments.setMap([current]);
    expect(part_at_hook(100, 99, 1)).toBe(100);
  });
});

describe("attachment_for_part", () => {
  // The door reads the module state object in its body (#387); seed the
  // world and reset it between tests.
  const a = att({
    part_id: 1,
    attached: "2020-01-01T00:00:00Z",
    detached: "2022-12-31T00:00:00Z",
  });

  beforeEach(() => {
    reset();
    attachments.setMap([a]);
  });

  it("finds the attachment of a part at a time", () => {
    // The collection's prep function rebuilds the attachment, so the door
    // returns a value-equal attachment, not the seeded instance.
    expect(attachment_for_part(1, new Date("2021-06-01T00:00:00Z"))).toEqual(a);
  });

  it("returns undefined if the part was not attached at that time", () => {
    expect(
      attachment_for_part(1, new Date("2019-01-01T00:00:00Z")),
    ).toBeUndefined();
  });

  it("matches only the requested part", () => {
    expect(
      attachment_for_part(999, new Date("2021-06-01T00:00:00Z")),
    ).toBeUndefined();
  });

  it("finds the attachment at the exact attached boundary (inclusive)", () => {
    expect(attachment_for_part(1, new Date("2020-01-01T00:00:00Z"))).toEqual(a);
  });

  it("does not find the attachment at the exact detached boundary (exclusive)", () => {
    expect(
      attachment_for_part(1, new Date("2022-12-31T00:00:00Z")),
    ).toBeUndefined();
  });
});

describe("mounted_on", () => {
  const gear = new Part({
    id: 100,
    owner: 1,
    what: 1,
    name: "Bike",
    purchase: "2023-01-01T00:00:00Z",
    last_used: "2023-01-01T00:00:00Z",
    usage: "u1",
  });
  const a = att({
    part_id: 1,
    attached: "2020-01-01T00:00:00Z",
    detached: "2022-12-31T00:00:00Z",
    gear: 100,
  });

  beforeEach(() => {
    reset();
    parts.setMap([]);
  });

  it("returns the gear the part is attached to at a time", () => {
    parts.setMap([gear]);
    attachments.setMap([a]);
    // The collection's prep function rebuilds the part, so the door returns
    // a value-equal part, not the seeded instance.
    expect(mounted_on(1, new Date("2021-06-01T00:00:00Z"))).toEqual(gear);
  });

  it("returns the gear at the exact attached boundary (inclusive)", () => {
    parts.setMap([gear]);
    attachments.setMap([a]);
    expect(mounted_on(1, new Date("2020-01-01T00:00:00Z"))).toEqual(gear);
  });

  it("returns undefined at the exact detached boundary (exclusive)", () => {
    parts.setMap([gear]);
    attachments.setMap([a]);
    expect(mounted_on(1, new Date("2022-12-31T00:00:00Z"))).toBeUndefined();
  });

  it("returns undefined if the part was not attached at that time", () => {
    parts.setMap([gear]);
    attachments.setMap([a]);
    expect(mounted_on(1, new Date("2019-01-01T00:00:00Z"))).toBeUndefined();
  });

  it("matches only the requested part", () => {
    parts.setMap([gear]);
    attachments.setMap([a]);
    expect(mounted_on(999, new Date("2021-06-01T00:00:00Z"))).toBeUndefined();
  });

  it("returns undefined when the gear is missing from the parts collection", () => {
    attachments.setMap([a]);
    expect(mounted_on(1, new Date("2021-06-01T00:00:00Z"))).toBeUndefined();
  });

  it("returns undefined for an undefined part id", () => {
    parts.setMap([gear]);
    attachments.setMap([a]);
    expect(
      mounted_on(undefined, new Date("2021-06-01T00:00:00Z")),
    ).toBeUndefined();
  });
});

describe("attacheesForGear", () => {
  it("returns all attachments of a gear at a time", () => {
    const a = att({
      part_id: 1,
      attached: "2020-01-01T00:00:00Z",
      detached: "2022-12-31T00:00:00Z",
      gear: 100,
    });
    const b = att({ part_id: 2, attached: "2021-06-01T00:00:00Z", gear: 100 });
    const res = attacheesForGear(
      100,
      attMap(a, b),
      new Date("2021-09-01T00:00:00Z"),
    );
    expect(res.map((x) => x.part_id).sort()).toEqual([1, 2]);
  });

  it("excludes attachments outside the window", () => {
    const a = att({
      part_id: 1,
      attached: "2020-01-01T00:00:00Z",
      detached: "2022-12-31T00:00:00Z",
      gear: 100,
    });
    expect(
      attacheesForGear(100, attMap(a), new Date("2023-06-01T00:00:00Z")),
    ).toEqual([]);
  });

  it("includes an attachment at the exact attached boundary (inclusive)", () => {
    const a = att({
      part_id: 1,
      attached: "2020-01-01T00:00:00Z",
      detached: "2022-12-31T00:00:00Z",
      gear: 100,
    });
    expect(
      attacheesForGear(100, attMap(a), new Date("2020-01-01T00:00:00Z")),
    ).toEqual([a]);
  });

  it("excludes an attachment at the exact detached boundary (exclusive)", () => {
    const a = att({
      part_id: 1,
      attached: "2020-01-01T00:00:00Z",
      detached: "2022-12-31T00:00:00Z",
      gear: 100,
    });
    expect(
      attacheesForGear(100, attMap(a), new Date("2022-12-31T00:00:00Z")),
    ).toEqual([]);
  });
});
