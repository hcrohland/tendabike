import { describe, expect, it, vi } from "vitest";
import { getTypes, types, Type, category } from "./types";
import { Activity } from "./activity";
import { Part } from "./part";
import { get } from "svelte/store";
import type { Map } from "./mapable";
import { resp } from "../test/helpers";

function partTypes(): any[] {
  return [
    { id: 1, name: "Bike", main: 1, hooks: [1, 2], order: 1, group: "gear" },
    {
      id: 10,
      name: "Wheel front",
      main: 1,
      hooks: [],
      order: 2,
      group: "wheels",
    },
    {
      id: 20,
      name: "Wheel rear",
      main: 1,
      hooks: [],
      order: 3,
      group: "wheels",
    },
    {
      id: 11,
      name: "Wheel front disc",
      main: 10,
      hooks: [],
      order: 4,
      group: "wheels",
    },
    {
      id: 9999,
      name: "Mystery Part",
      main: 9999,
      hooks: [],
      order: 99,
      group: undefined,
    },
  ];
}

function actTypes(): any[] {
  return [
    { id: 301, name: "Ride", gear_type: 1 },
    { id: 302, name: "Wheel Ride", gear_type: 10 },
  ];
}

async function loadTypes() {
  const fetchMock = vi.fn();
  fetchMock
    .mockResolvedValueOnce(resp(partTypes()))
    .mockResolvedValueOnce(resp(actTypes()));
  vi.stubGlobal("fetch", fetchMock);
  await getTypes();
}

describe("getTypes", () => {
  it("fetches part and activity types and populates the types map", async () => {
    await loadTypes();
    const t = types as Map<Type>;
    expect(t[1]).toBeInstanceOf(Type);
    expect(t[1].name).toBe("Bike");
    expect(t[10].name).toBe("Wheel front");
    expect(t[20].name).toBe("Wheel rear");
  });

  it("attaches activity types to the matching part type", async () => {
    await loadTypes();
    const t = types as Map<Type>;
    expect(t[1].acts).toHaveLength(1);
    expect(t[1].acts[0].id).toBe(301);
    expect(t[10].acts).toHaveLength(1);
    expect(t[10].acts[0].id).toBe(302);
  });

  it("sets the category store to type 1", async () => {
    await loadTypes();
    const cat = get(category);
    expect(cat.id).toBe(1);
    expect(cat.name).toBe("Bike");
  });
});

describe("Type.activities", () => {
  it("returns activities matching the type's acts, sorted by start desc", async () => {
    await loadTypes();
    const t = types as Map<Type>;
    const acts = {
      1: new Activity({
        id: 1,
        what: 301,
        start: "2024-01-01T00:00:00Z",
        gear: null,
        name: "A",
      }),
      2: new Activity({
        id: 2,
        what: 301,
        start: "2024-06-01T00:00:00Z",
        gear: null,
        name: "B",
      }),
      3: new Activity({
        id: 3,
        what: 302,
        start: "2024-03-01T00:00:00Z",
        gear: null,
        name: "C",
      }),
    } as Map<Activity>;
    const res = t[1].activities(acts);
    expect(res.map((a) => a.id)).toEqual([2, 1]);
  });
});

describe("Type.parts", () => {
  it("returns parts with matching what, sorted by last_used desc", async () => {
    await loadTypes();
    const t = types as Map<Type>;
    const parts = {
      1: new Part({
        id: 1,
        what: 10,
        last_used: "2024-01-01T00:00:00Z",
        name: "A",
        owner: 1,
        purchase: "2023-01-01T00:00:00Z",
        usage: "u1",
      }),
      2: new Part({
        id: 2,
        what: 10,
        last_used: "2024-06-01T00:00:00Z",
        name: "B",
        owner: 1,
        purchase: "2023-01-01T00:00:00Z",
        usage: "u1",
      }),
      3: new Part({
        id: 3,
        what: 20,
        last_used: "2024-03-01T00:00:00Z",
        name: "C",
        owner: 1,
        purchase: "2023-01-01T00:00:00Z",
        usage: "u1",
      }),
    } as Map<Part>;
    const res = t[10].parts(parts);
    expect(res.map((p) => p.id)).toEqual([2, 1]);
  });
});

describe("Type.localizedName", () => {
  it("falls back to the raw name when no key exists", async () => {
    await loadTypes();
    const t = types as Map<Type>;
    expect(t[9999].localizedName()).toBe("Mystery Part");
  });
});

describe("Type.human_name", () => {
  it("returns localizedName for null hook", async () => {
    await loadTypes();
    const t = types as Map<Type>;
    expect(t[10].human_name(null)).toBe(t[10].localizedName());
  });
});

describe("Type.subtypes", () => {
  it("returns types where main == this.id, sorted by order", async () => {
    await loadTypes();
    const t = types as Map<Type>;
    const subs = t[10].subtypes();
    expect(subs.map((s) => s.id)).toEqual([11]);
    expect(subs[0].name).toBe("Wheel front disc");
  });
});

describe("Type.is_hook", () => {
  it("returns true if any type includes this id in its hooks", async () => {
    await loadTypes();
    const t = types as Map<Type>;
    // type 1 has hooks [1, 2], so type 1 is a hook
    expect(t[1].is_hook()).toBe(true);
    // type 9999 is not referenced by any type's hooks
    expect(t[9999].is_hook()).toBe(false);
  });
});
