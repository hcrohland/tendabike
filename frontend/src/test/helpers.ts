import { expect, vi } from "vitest";
import { Activity } from "../lib/activity";
import { Part } from "../lib/part";
import { Usage } from "../lib/usage";
import { type Color } from "../Widgets/color";

export function resp(body: any, status = 200, ok = true, statusText = "") {
  return {
    ok,
    status,
    statusText,
    json: async () => body,
    text: async () => (typeof body === "string" ? body : JSON.stringify(body)),
  } as unknown as Response;
}

export function usage(id: string, o: Partial<any> = {}): Usage {
  return new Usage({
    id,
    count: 0,
    climb: 0,
    descend: 0,
    distance: 0,
    time: 0,
    duration: 0,
    energy: 0,
    ...o,
  });
}

/// The common Part seed data; callers extend it (a spare-type part adds
/// vendor/model).
export function partData(overrides: Partial<any> = {}): any {
  return {
    id: 7,
    owner: 1,
    what: 10,
    name: "Wheel",
    purchase: "2023-01-01T00:00:00Z",
    last_used: "2024-01-01T00:00:00Z",
    usage: "u1",
    ...overrides,
  };
}

/// The standard `u1` usage seed: one ride's worth (one ride, 1000 m, an hour,
/// 100 kJ).
export function standardUsage(overrides: Partial<any> = {}): Usage {
  return usage("u1", {
    count: 1,
    distance: 1000,
    time: 3600,
    duration: 3600,
    energy: 100,
    ...overrides,
  });
}

/// A fetch mock for the `/api/types/*` endpoints the part components load:
/// one part type ("Wheel"), no activity types.
export function typesFetch() {
  return vi.fn((url: string) => {
    if (url.includes("/api/types/part")) {
      return Promise.resolve(
        resp([{ id: 10, name: "Wheel", main: 1, hooks: [1], order: 1 }]),
      );
    }
    if (url.includes("/api/types/activity")) {
      return Promise.resolve(resp([]));
    }
    return Promise.resolve(resp(null));
  });
}

/// The shared Activity fixture.
export function activity(): Activity {
  return new Activity({
    id: "icu:100",
    user_id: 1,
    what: 301,
    name: "Ride",
    start: "2024-05-01T08:00:00Z",
    distance: 20000,
    time: 3600,
    duration: 3600,
    climb: 100,
    descend: 80,
    energy: 50,
  });
}

/// Assert that `el` renders the given color's surface role class and not the
/// other color's.
export function expectSurface(el: Element | null | undefined, color: Color) {
  const other: Color = color === "primary" ? "secondary" : "primary";
  expect(el).toBeTruthy();
  expect(el!.classList.contains(`surface-${color}`)).toBe(true);
  expect(el!.classList.contains(`surface-${other}`)).toBe(false);
}

/// Assert that a chip's `value` text sits on the theme's single uniform fill:
/// its nearest `.fill` ancestor is the fill (not a surface role class) and
/// the text is white.
export function expectFill(value: Element | null | undefined) {
  expect(value).toBeTruthy();
  const fill = value!.closest(".fill");
  expect(fill).toBeTruthy();
  expect(fill!.classList.contains("surface-primary")).toBe(false);
  expect(fill!.classList.contains("surface-secondary")).toBe(false);
  expect(value!.classList.contains("text-white")).toBe(true);
}

/// The all-empty Summary fixture: every one of the nine collections an empty
/// object, accepting per-field overrides. The wire shape is a uniform object
/// with stringified id keys and `null` for tombstones.
export function summary(overrides: Partial<any> = {}): any {
  return {
    parts: {},
    part_notes: {},
    attachments: {},
    activities: {},
    usages: {},
    services: {},
    plans: {},
    shops: {},
    users: {},
    notifications: {},
    ...overrides,
  };
}

/// The suite's content-rich Summary: one entry per populated collection.
export function summaryContent(): any {
  return summary({
    parts: {
      "1": {
        id: 1,
        owner: 1,
        what: 10,
        name: "P1",
        vendor: "",
        model: "",
        purchase: "2023-01-01T00:00:00Z",
        last_used: "2024-01-01T00:00:00Z",
        disposed_at: null,
        usage: "u1",
        shop: null,
      },
    },
    part_notes: {},
    attachments: {},
    activities: {
      "100": {
        id: 100,
        user_id: 1,
        what: 301,
        name: "Ride",
        start: "2024-05-01T08:00:00Z",
        gear: null,
        climb: 0,
        descend: 0,
        distance: 1000,
        time: 3600,
        duration: 3600,
        energy: 100,
        device_name: "",
      },
    },
    usages: {
      u1: {
        id: "u1",
        count: 1,
        climb: 0,
        descend: 0,
        distance: 1000,
        time: 3600,
        duration: 3600,
        energy: 100,
      },
    },
    services: {
      S1: {
        id: "S1",
        part_id: 1,
        time: "2023-01-01T00:00:00Z",
        redone: "2023-01-01T00:00:00Z",
        name: "Svc",
        notes: "",
        usage: "u1",
        successor: null,
        plans: [],
      },
    },
    plans: {
      PL1: {
        id: "PL1",
        part: 1,
        what: 10,
        hook: null,
        name: "Plan",
        days: null,
        hours: null,
        km: "100",
        climb: null,
        descend: null,
        rides: null,
        kJ: null,
      },
    },
    shops: {
      "10": {
        id: 10,
        owner: 1,
        name: "Shop",
        description: "",
        auto_approve: true,
        created_at: "2023-01-01T00:00:00Z",
      },
    },
    users: {
      "1": {
        id: 1,
        firstname: "Max",
        name: "Max Mustermann",
        avatar: undefined,
      },
    },
  });
}
