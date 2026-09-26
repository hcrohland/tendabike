import { describe, expect, it, vi, beforeEach } from "vitest";
import {
  Limits,
  ServicePlan,
  localizeLimitKey,
  plans,
  type limit_keys,
} from "./serviceplan";
import { Part } from "./part";
import { Service, services } from "./service";
import { resp } from "../test/helpers";

function plan(overrides: Partial<any> = {}): ServicePlan {
  return new ServicePlan({
    id: "P1",
    part: 100,
    what: 10,
    hook: null,
    name: "Plan",
    ...overrides,
  });
}

function part(overrides: Partial<any> = {}): Part {
  return new Part({
    id: 5,
    owner: 1,
    what: 10,
    name: "Wheel",
    purchase: "2023-01-01T00:00:00Z",
    last_used: "2023-01-01T00:00:00Z",
    usage: "u1",
    ...overrides,
  });
}

function svc(overrides: Partial<any> = {}): Service {
  return new Service({
    id: "S1",
    part_id: 5,
    time: "2023-01-01T00:00:00Z",
    redone: "2023-01-01T00:00:00Z",
    name: "Service",
    notes: "",
    usage: "u1",
    successor: null,
    plans: [],
    ...overrides,
  });
}

describe("Limits", () => {
  it("parses numeric strings into integers", () => {
    const l = new Limits({ days: "30", km: "100", rides: "5" });
    expect(l.days).toBe(30);
    expect(l.km).toBe(100);
    expect(l.rides).toBe(5);
  });

  it("treats empty, zero and missing values as null", () => {
    const l = new Limits({ days: "", km: "0", rides: null });
    expect(l.days).toBeNull();
    expect(l.km).toBeNull();
    expect(l.rides).toBeNull();
    expect(l.descend).toBeNull();
  });

  it("exposes the limit keys", () => {
    expect(Limits.keys).toEqual([
      "days",
      "rides",
      "hours",
      "km",
      "climb",
      "descend",
      "kJ",
    ]);
  });

  it("is valid when any limit is positive", () => {
    expect(Limits.valid(new Limits({ km: "100" }))).toBe(true);
  });

  it("is invalid when no limit is set", () => {
    expect(Limits.valid(new Limits({}))).toBe(false);
  });

  it("is invalid when limits are only zero", () => {
    expect(Limits.valid(new Limits({ km: "0", rides: "0" }))).toBe(false);
  });

  it("to_object returns the limit fields", () => {
    expect(new Limits({ km: "100" }).to_object()).toEqual({
      days: null,
      hours: null,
      km: 100,
      climb: null,
      descend: null,
      rides: null,
      kJ: null,
    });
  });

  it("set_from_object copies fields onto the instance", () => {
    const l = new Limits({});
    l.set_from_object({ km: 5, rides: 3 });
    expect(l.km).toBe(5);
    expect(l.rides).toBe(3);
    expect(l.days).toBeNull();
  });
});

describe("ServicePlan.valid", () => {
  it("is true when a limit, name and what are set", () => {
    expect(plan({ name: "X", what: 10, km: "100" }).valid()).toBe(true);
  });

  it("is false when the name is empty", () => {
    expect(plan({ name: "", what: 10, km: "100" }).valid()).toBe(false);
  });

  it("is false when what is undefined", () => {
    expect(plan({ name: "X", km: "100", what: undefined }).valid()).toBe(false);
  });

  it("is false when no limit is set", () => {
    expect(plan({ name: "X", what: 10 }).valid()).toBe(false);
  });
});

describe("ServicePlan.services", () => {
  // The method reads the module's services state object in its body (issue
  // #374); seed the world and reset it between tests.
  beforeEach(() => {
    services.setMap([]);
  });

  it("returns only services for this part and plan, newest first", () => {
    services.setMap([
      svc({
        id: "S1",
        part_id: 5,
        plans: ["P1"],
        time: "2023-01-01T00:00:00Z",
      }),
      svc({
        id: "S2",
        part_id: 5,
        plans: ["P1"],
        time: "2024-01-01T00:00:00Z",
      }),
      svc({
        id: "S3",
        part_id: 5,
        plans: ["OTHER"],
        time: "2023-06-01T00:00:00Z",
      }),
      svc({
        id: "S4",
        part_id: 99,
        plans: ["P1"],
        time: "2023-06-01T00:00:00Z",
      }),
    ]);
    const res = plan({ id: "P1" }).services(part({ id: 5 }));
    expect(res.map((s) => s.id)).toEqual(["S2", "S1"]);
  });

  it("returns an empty list when the part is null", () => {
    expect(plan({ id: "P1" }).services(null)).toEqual([]);
  });
});

describe("localizeLimitKey", () => {
  it("returns the localized label for a known key", () => {
    expect(localizeLimitKey("kJ")).toBe("energy");
  });

  it("falls back to the key for an unknown key", () => {
    expect(localizeLimitKey("bogus" as limit_keys)).toBe("bogus");
  });
});

describe("ServicePlan CRUD", () => {
  let fetchMock: ReturnType<typeof vi.fn>;

  beforeEach(() => {
    plans.setMap([]);
    services.setMap([]);
    fetchMock = vi.fn();
    vi.stubGlobal("fetch", fetchMock);
  });

  it("ServicePlan.create POSTs to /api/plan and updates the plans store", async () => {
    const p = plan({
      id: "00000000-0000-0000-0000-000000000000",
      name: "Test Plan",
      what: 10,
      part: 5,
      km: "100",
    });
    fetchMock.mockResolvedValueOnce(
      resp({
        id: "NEW1",
        name: "Test Plan",
        what: 10,
        part: 5,
        hook: null,
        km: 100,
        days: null,
        hours: null,
        climb: null,
        descend: null,
        rides: null,
        kJ: null,
      }),
    );
    await p.create();
    expect(fetchMock).toHaveBeenCalledTimes(1);
    const [url, options] = fetchMock.mock.calls[0];
    expect(url).toBe("/api/plan");
    expect(options.method).toBe("POST");
    expect(plans["NEW1"]).toBeDefined();
  });

  it("ServicePlan.update PUTs to /api/plan and updates the plans store", async () => {
    const p = plan({ id: "P1", name: "Updated", what: 10, part: 5 });
    fetchMock.mockResolvedValueOnce(
      resp({
        id: "P1",
        name: "Updated",
        what: 10,
        part: 5,
        hook: null,
        km: null,
        days: null,
        hours: null,
        climb: null,
        descend: null,
        rides: null,
        kJ: null,
      }),
    );
    await p.update();
    expect(fetchMock).toHaveBeenCalledTimes(1);
    const [url, options] = fetchMock.mock.calls[0];
    expect(url).toBe("/api/plan");
    expect(options.method).toBe("PUT");
    expect(plans["P1"].name).toBe("Updated");
  });

  it("ServicePlan.delete removes the plan and updates services", async () => {
    plans.updateMap([plan({ id: "P1" })]);
    services.updateMap([
      new Service({
        id: "S1",
        part_id: 5,
        time: "2023-01-01T00:00:00Z",
        redone: "2023-01-01T00:00:00Z",
        name: "Svc",
        notes: "",
        usage: "u1",
        successor: null,
        plans: [],
      }),
    ]);
    fetchMock.mockResolvedValueOnce(resp([]));
    await plan({ id: "P1" }).delete();
    expect(fetchMock).toHaveBeenCalledTimes(1);
    const [url, options] = fetchMock.mock.calls[0];
    expect(url).toBe("/api/plan/P1");
    expect(options.method).toBe("DELETE");
    expect(plans["P1"]).toBeUndefined();
    expect(services["S1"]).toBeDefined();
  });
});
