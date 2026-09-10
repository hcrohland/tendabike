import { describe, expect, it, vi, beforeEach } from "vitest";
import {
  Limits,
  ServicePlan,
  localizeLimitKey,
  next_due,
  plans_for_part,
  plans,
  type limit_keys,
} from "./serviceplan";
import { Part } from "./part";
import { Attachment } from "./attachment";
import { Service, services } from "./service";
import { Usage } from "./usage";
import { maxDate } from "./store";
import { type Map } from "./mapable";
import { get } from "svelte/store";
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

function planMap(...ps: ServicePlan[]): Map<ServicePlan> {
  return Object.fromEntries(ps.map((p) => [p.id!, p])) as Map<ServicePlan>;
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

function svcMap(...ss: Service[]): Map<Service> {
  return Object.fromEntries(ss.map((s) => [s.id!, s])) as Map<Service>;
}

function usage(id: string, o: Partial<any> = {}): Usage {
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

function usageMap(...us: Usage[]): Map<Usage> {
  return Object.fromEntries(us.map((u) => [u.id, u])) as Map<Usage>;
}

function att(overrides: Partial<any> = {}): Attachment {
  return new Attachment({
    part_id: 7,
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

function attMap(...as: Attachment[]): Map<Attachment> {
  return Object.fromEntries(as.map((a) => [a.idx, a])) as Map<Attachment>;
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
  const map = svcMap(
    svc({ id: "S1", part_id: 5, plans: ["P1"], time: "2023-01-01T00:00:00Z" }),
    svc({ id: "S2", part_id: 5, plans: ["P1"], time: "2024-01-01T00:00:00Z" }),
    svc({
      id: "S3",
      part_id: 5,
      plans: ["OTHER"],
      time: "2023-06-01T00:00:00Z",
    }),
    svc({ id: "S4", part_id: 99, plans: ["P1"], time: "2023-06-01T00:00:00Z" }),
  );

  it("returns only services for this part and plan, newest first", () => {
    const res = plan({ id: "P1" }).services(part({ id: 5 }), map);
    expect(res.map((s) => s.id)).toEqual(["S2", "S1"]);
  });

  it("returns an empty list when the part is null", () => {
    expect(plan({ id: "P1" }).services(null, map)).toEqual([]);
  });
});

describe("ServicePlan.getpart", () => {
  const part7 = part({ id: 7 });
  const part100 = part({ id: 100 });
  const parts = { 7: part7, 100: part100 } as Map<Part>;

  it("resolves the attached part at the hook", () => {
    const res = plan({ part: 100, what: 10, hook: 1 }).getpart(
      parts,
      attMap(att()),
    );
    expect(res).toBe(part7);
  });

  it("falls back to the gear part when nothing is attached", () => {
    const res = plan({ part: 100, what: 10, hook: 1 }).getpart(parts, {});
    expect(res).toBe(part100);
  });

  it("returns null when there is no part", () => {
    expect(plan({ part: null }).getpart(parts, {})).toBeNull();
  });
});

describe("ServicePlan.due", () => {
  const p = part({ id: 5, what: 10, usage: "u_now" });
  const u_prev = usage("u_prev", {
    count: 2,
    distance: 1000,
    climb: 10,
    descend: 5,
    time: 3600,
    energy: 100,
  });
  const u_now = usage("u_now", {
    count: 4,
    distance: 40000,
    climb: 40,
    descend: 20,
    time: 18000,
    energy: 1000,
  });
  const usages = usageMap(u_prev, u_now);

  it("subtracts usage since the last service", () => {
    const pln = plan({
      what: 10,
      km: "100",
      rides: "10",
      climb: "50",
      descend: "20",
      kJ: "1000",
      hours: "5",
    });
    const service = svc({
      id: "S1",
      usage: "u_prev",
      time: "2024-01-01T00:00:00Z",
    });
    const due = pln.due(p, service, usages);
    expect(due.km).toBe(61);
    expect(due.rides).toBe(8);
    expect(due.climb).toBe(20);
    expect(due.descend).toBe(5);
    expect(due.kJ).toBe(100);
    expect(due.hours).toBe(1);
    expect(due.days).toBeNull();
  });

  it("uses the full usage when there is no service", () => {
    const pln = plan({ what: 10, km: "100", rides: "10" });
    const due = pln.due(p, undefined, usages);
    expect(due.km).toBe(60);
    expect(due.rides).toBe(6);
    expect(due.climb).toBeNull();
  });

  it("returns empty limits when the part type differs", () => {
    const pln = plan({ what: 10, km: "100" });
    const due = pln.due(part({ what: 99 }), undefined, usages);
    expect(due.km).toBeNull();
    expect(due.to_object()).toEqual({
      days: null,
      hours: null,
      km: null,
      climb: null,
      descend: null,
      rides: null,
      kJ: null,
    });
  });

  it("returns empty limits for a null part", () => {
    const due = plan({ what: 10, km: "100" }).due(null, undefined, usages);
    expect(due.km).toBeNull();
  });
});

describe("ServicePlan.alert", () => {
  const p = part({ id: 5, what: 10, usage: "u_now" });
  const pln = plan({ what: 10, km: "100" });

  it("is 'warn' when the due value is close to zero", () => {
    const usages = usageMap(usage("u_now", { distance: 97000 }));
    expect(pln.alert(p, undefined, usages)).toBe("warn");
  });

  it("is 'alert' when the due value is negative", () => {
    const usages = usageMap(usage("u_now", { distance: 110000 }));
    expect(pln.alert(p, undefined, usages)).toBe("alert");
  });

  it("is empty when the service is not due soon", () => {
    const usages = usageMap(usage("u_now", { distance: 50000 }));
    expect(pln.alert(p, undefined, usages)).toBe("");
  });

  it("is empty when the part does not match", () => {
    const usages = usageMap(usage("u_now", { distance: 97000 }));
    expect(pln.alert(part({ what: 99 }), undefined, usages)).toBe("");
  });
});

describe("ServicePlan.no_template", () => {
  it("returns the part of the plan when it exists", () => {
    const p = plan({ id: "P1", part: 5 });
    expect(p.no_template(planMap(p))).toBe(5);
  });

  it("returns undefined when the plan is absent", () => {
    const p = plan({ id: "P1", part: 5 });
    expect(p.no_template(planMap())).toBeUndefined();
  });
});

describe("ServicePlan.partLink", () => {
  it("returns '' when there is no part", () => {
    expect(plan({ part: null }).partLink({})).toBe("");
  });

  it("returns the part link", () => {
    const parts = { 5: part({ id: 5, name: "Wheel" }) } as Map<Part>;
    expect(plan({ part: 5 }).partLink(parts)).toBe(
      `<a href="/#/part/5" style="text-decoration:none" class="text-reset">Wheel</a>`,
    );
  });
});

describe("next_due", () => {
  const p = part({ id: 5, what: 10, usage: "u_now" });
  const usages = usageMap(usage("u_now", { distance: 40000 }));

  it("tracks the plan with the smallest due per limit", () => {
    const a = plan({ id: "A", what: 10, km: "200" });
    const b = plan({ id: "B", what: 10, km: "100" });
    expect(next_due(p, [a, b], {}, usages)).toEqual({
      km: { due: 60, plan: 100 },
    });
  });

  it("returns an empty object when nothing is due", () => {
    const a = plan({ id: "A", what: 99, km: "100" });
    expect(next_due(p, [a], {}, usages)).toEqual({});
  });
});

describe("plans_for_part", () => {
  it("returns the part's own plans when it is not attached", () => {
    const plans = planMap(
      plan({ id: "P1", part: 5, hook: null }),
      plan({ id: "P2", part: 5, hook: 1 }),
      plan({ id: "P3", part: 99, hook: null }),
    );
    const res = plans_for_part(plans, {}, 5);
    expect(res.map((p) => p.id)).toEqual(["P1"]);
  });

  it("returns specific and gear-level plans for an attached part", () => {
    const plans = planMap(
      plan({ id: "P1", part: 7, hook: null }),
      plan({ id: "P2", part: 100, hook: 1, what: 10 }),
      plan({ id: "P3", part: 99, hook: 1, what: 10 }),
    );
    const res = plans_for_part(plans, attMap(att()), 7);
    expect(res.map((p) => p.id)).toEqual(["P1", "P2"]);
  });

  it("maps generic null-part plans onto the attached part", () => {
    const generic = plan({
      id: "P1",
      part: null,
      hook: 1,
      what: 10,
      km: "100",
    });
    const plans = planMap(
      generic,
      plan({ id: "P2", part: null, hook: 2, what: 10 }),
      plan({ id: "P3", part: null, hook: 1, what: 20 }),
      plan({ id: "P4", part: 99, hook: 1, what: 10 }),
    );
    const res = plans_for_part(plans, attMap(att()), 7);
    expect(res).toHaveLength(1);
    expect(res[0].part).toBe(7);
    expect(res[0].what).toBe(10);
    expect(res[0].hook).toBe(1);
    expect(res[0]).not.toBe(generic);
  });

  it("treats a not-yet-attached part as unattached at the given time", () => {
    const plans = planMap(plan({ id: "P1", part: 5, hook: null }));
    const atts = attMap(
      att({
        part_id: 5,
        attached: "2030-01-01T00:00:00Z",
        detached: maxDate.getTime(),
      }),
    );
    const res = plans_for_part(
      plans,
      atts,
      5,
      new Date("2029-01-01T00:00:00Z"),
    );
    expect(res.map((p) => p.id)).toEqual(["P1"]);
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
    expect(get(plans)["NEW1"]).toBeDefined();
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
    expect(get(plans)["P1"].name).toBe("Updated");
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
    expect(get(plans)["P1"]).toBeUndefined();
    expect(get(services)["S1"]).toBeDefined();
  });
});
