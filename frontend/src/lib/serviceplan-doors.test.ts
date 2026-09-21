import { describe, expect, it, vi } from "vitest";
import {
  partForPlanGear,
  duesForPlans,
  gearsForPlan,
  alertCounts,
  plansForPart,
  plansForAssembly,
  isTemplate,
  localizeLimitKey,
  ServicePlan,
  type limit_keys,
} from "./serviceplan";
import { Part } from "./part";
import { Attachment } from "./attachment";
import { Service } from "./service";
import { Usage } from "./usage";
import { getTypes } from "./types";
import { maxDate } from "./store";
import { type Map } from "./mapable";
import { resp } from "../test/helpers";

const wheelType = {
  id: 10,
  name: "Wheel front",
  main: 10,
  hooks: [30, 31],
  order: 2,
  group: "wheels",
};

// Assembly-walk fixture: the bike assembles wheels through subtype hooks 30/31.
const bikeType = {
  id: 1,
  name: "Bike",
  main: 1,
  hooks: [1, 2],
  order: 1,
  group: "gear",
};
const wheelAsmType = { ...wheelType, main: 1 };

// Door test suite for the new public surface of serviceplan.ts (issue #327).
// Fixture helpers are copied from serviceplan.test.ts on purpose: importing
// that file would re-run its describes.

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

function planMap(...ps: ServicePlan[]): Map<ServicePlan> {
  return Object.fromEntries(ps.map((p) => [p.id!, p])) as Map<ServicePlan>;
}

/** Populate the live `types` binding by stubbing fetch and calling
 * getTypes() (the module binding is not assignable from a test file).
 */
async function loadTypes(partTypes: any[]) {
  const fetchMock = vi.fn();
  fetchMock
    .mockResolvedValueOnce(resp(partTypes))
    .mockResolvedValueOnce(resp([]));
  vi.stubGlobal("fetch", fetchMock);
  await getTypes();
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

describe("partForPlanGear", () => {
  const part7 = part({ id: 7 });
  const part100 = part({ id: 100 });
  const parts = { 7: part7, 100: part100 } as Map<Part>;

  it("resolves the attached part at the hook", () => {
    const res = partForPlanGear(
      plan({ part: 100, what: 10, hook: 1 }),
      undefined,
      parts,
      attMap(att()),
    );
    expect(res).toBe(part7);
  });

  it("falls back to the gear part when nothing is attached", () => {
    const res = partForPlanGear(
      plan({ part: 100, what: 10, hook: 1 }),
      undefined,
      parts,
      {},
    );
    expect(res).toBe(part100);
  });

  it("prefers the explicit gear over the plan's part", () => {
    const res = partForPlanGear(
      plan({ part: 100, what: 10, hook: 1 }),
      7,
      parts,
      {},
    );
    expect(res).toBe(part7);
  });

  it("returns null when there is no part", () => {
    expect(
      partForPlanGear(plan({ part: null }), undefined, parts, {}),
    ).toBeNull();
  });
});

describe("duesForPlans", () => {
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
      plans: ["P1"],
    });
    expect(duesForPlans(p, [pln], svcMap(service), usages)).toEqual({
      rides: { due: 8, plan: 10 },
      hours: { due: 1, plan: 5 },
      km: { due: 61, plan: 100 },
      climb: { due: 20, plan: 50 },
      descend: { due: 5, plan: 20 },
      kJ: { due: 100, plan: 1000 },
    });
  });

  it("uses the full usage when there is no service", () => {
    const pln = plan({ what: 10, km: "100", rides: "10" });
    expect(duesForPlans(p, [pln], {}, usages)).toEqual({
      rides: { due: 6, plan: 10 },
      km: { due: 60, plan: 100 },
    });
  });

  it("tracks the plan with the smallest due per limit", () => {
    const a = plan({ id: "A", what: 10, km: "200" });
    const b = plan({ id: "B", what: 10, km: "100" });
    expect(duesForPlans(p, [a, b], {}, usages)).toEqual({
      km: { due: 60, plan: 100 },
    });
  });

  it("returns an empty object when nothing is due", () => {
    const a = plan({ what: 99, km: "100" });
    expect(duesForPlans(p, [a], {}, usages)).toEqual({});
  });

  it("returns an empty object when the part type differs", () => {
    expect(
      duesForPlans(part({ what: 99 }), [plan({ km: "100" })], {}, usages),
    ).toEqual({});
  });

  it("returns an empty object for a null part", () => {
    expect(duesForPlans(null, [plan({ km: "100" })], {}, usages)).toEqual({});
  });
});

describe("gearsForPlan", () => {
  it("resolves a specific plan to its own part", () => {
    const p5 = part({ id: 5 });
    const pln = plan({ id: "P1", part: 5, what: 10, hook: null });
    expect(gearsForPlan(pln, { 5: p5 }, {}, planMap(pln))).toEqual([p5]);
  });

  it("resolves a generic plan to all active parts of the type", async () => {
    await loadTypes([wheelType]);
    const p5 = part({ id: 5, what: 10 });
    const p7 = part({ id: 7, what: 10 });
    const G = plan({ id: "G", part: null, hook: 30, what: 10, km: "100" });
    expect(gearsForPlan(G, { 5: p5, 7: p7 }, {}, planMap(G))).toEqual([p5, p7]);
  });

  it("excludes disposed parts", async () => {
    await loadTypes([wheelType]);
    const p5 = part({ id: 5, what: 10, disposed_at: "2024-01-01T00:00:00Z" });
    const p7 = part({ id: 7, what: 10 });
    const G = plan({ id: "G", part: null, hook: 30, what: 10, km: "100" });
    expect(gearsForPlan(G, { 5: p5, 7: p7 }, {}, planMap(G))).toEqual([p7]);
  });

  it("excludes parts covered by a dedicated plan", async () => {
    await loadTypes([wheelType]);
    const p5 = part({ id: 5, what: 10, usage: "u5" });
    const p7 = part({ id: 7, what: 10, usage: "u7" });
    const atts = attMap(att({ part_id: 7, gear: 5, hook: 30, what: 10 }));
    const G = plan({ id: "G", part: null, hook: 30, what: 10, km: "100" });
    const P7 = plan({ id: "P7", part: 7, hook: null, what: 10, km: "10" });
    expect(gearsForPlan(G, { 5: p5, 7: p7 }, atts, planMap(G, P7))).toEqual([
      p7,
    ]);
  });
});

describe("alertCounts", () => {
  const p = part({ id: 5, what: 10, usage: "u_now" });

  it("counts 'warn' when the due value is close to zero", () => {
    const usages = usageMap(usage("u_now", { distance: 97000 }));
    expect(
      alertCounts(
        [plan({ part: 5, what: 10, km: "100" })],
        { 5: p },
        {},
        usages,
        {},
      ),
    ).toEqual({ warn: 1, alert: 0 });
  });

  it("counts 'alert' when the due value is negative", () => {
    const usages = usageMap(usage("u_now", { distance: 110000 }));
    expect(
      alertCounts(
        [plan({ part: 5, what: 10, km: "100" })],
        { 5: p },
        {},
        usages,
        {},
      ),
    ).toEqual({ warn: 0, alert: 1 });
  });

  it("counts nothing when the service is not due soon", () => {
    const usages = usageMap(usage("u_now", { distance: 50000 }));
    expect(
      alertCounts(
        [plan({ part: 5, what: 10, km: "100" })],
        { 5: p },
        {},
        usages,
        {},
      ),
    ).toEqual({ warn: 0, alert: 0 });
  });

  it("counts nothing when the part does not match", () => {
    const usages = usageMap(usage("u_now", { distance: 97000 }));
    expect(
      alertCounts(
        [plan({ part: 5, what: 99, km: "100" })],
        { 5: p },
        {},
        usages,
        {},
      ),
    ).toEqual({ warn: 0, alert: 0 });
  });

  it("does not count parts covered by a dedicated plan", async () => {
    await loadTypes([wheelType]);
    const p5 = part({ id: 5, what: 10, usage: "u5" });
    const p7 = part({ id: 7, what: 10, usage: "u7" });
    const usages = usageMap(
      usage("u5", { distance: 97000 }),
      usage("u7", { distance: 97000 }),
    );
    const atts = attMap(att({ part_id: 7, gear: 5, hook: 30, what: 10 }));
    const G = plan({ id: "G", part: null, hook: 30, what: 10, km: "100" });
    const P7 = plan({ id: "P7", part: 7, hook: null, what: 10, km: "10" });
    expect(alertCounts([G, P7], { 5: p5, 7: p7 }, {}, usages, atts)).toEqual({
      warn: 1,
      alert: 1,
    });
  });
});

describe("plansForPart", () => {
  it("returns the part's own plans when it is not attached", () => {
    const plans = planMap(
      plan({ id: "P1", part: 5, hook: null }),
      plan({ id: "P2", part: 5, hook: 1 }),
      plan({ id: "P3", part: 99, hook: null }),
    );
    const res = plansForPart(5, plans, {});
    expect(res.map((p) => p.id)).toEqual(["P1"]);
  });

  it("returns specific and gear-level plans for an attached part", () => {
    const plans = planMap(
      plan({ id: "P1", part: 7, hook: null }),
      plan({ id: "P2", part: 100, hook: 1, what: 10 }),
      plan({ id: "P3", part: 99, hook: 1, what: 10 }),
    );
    const res = plansForPart(7, plans, attMap(att()));
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
    const res = plansForPart(7, plans, attMap(att()));
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
    const res = plansForPart(5, plans, atts, new Date("2029-01-01T00:00:00Z"));
    expect(res.map((p) => p.id)).toEqual(["P1"]);
  });

  it("returns the plans sorted by type, then part, then id", () => {
    const plans = planMap(
      plan({ id: "P1", part: 5, what: 20, hook: null }),
      plan({ id: "P2", part: 5, what: 10, hook: null }),
      plan({ id: "P3", part: 5, what: 10, hook: null }),
    );
    const res = plansForPart(5, plans, {});
    expect(res.map((p) => p.id)).toEqual(["P2", "P3", "P1"]);
  });

  it("returns the plans sorted by hook and part within a type", () => {
    // Part 7 is attached at hook 1, what 10 on gear 100 (see att()).
    const B = plan({ id: "B", part: 7, what: 10, hook: 1 });
    const C = plan({ id: "C", part: 100, what: 10, hook: 1 });
    const D = plan({ id: "D", part: 7, what: 10, hook: 30 });
    const plans = planMap(C, D, B);
    const res = plansForPart(7, plans, attMap(att()));
    expect(res.map((p) => p.id)).toEqual(["B", "C", "D"]);
  });
});

describe("plansForAssembly", () => {
  it("returns the gear's own plans plus the plans of attached parts", async () => {
    await loadTypes([bikeType, wheelAsmType]);
    const bike = part({ id: 100, what: 1 });
    const atts = attMap(att({ part_id: 5, gear: 100, hook: 30, what: 10 }));
    const P1 = plan({ id: "P1", part: 100, what: 1, hook: null, km: "100" });
    const P2 = plan({ id: "P2", part: 5, what: 10, hook: null, km: "100" });
    const P3 = plan({ id: "P3", part: 100, what: 10, hook: 30, km: "100" });
    const P4 = plan({ id: "P4", part: null, what: 10, hook: 30, km: "100" });
    const P5 = plan({ id: "P5", part: null, what: 10, hook: 31, km: "100" });
    const res = plansForAssembly(bike, planMap(P1, P2, P3, P4, P5), atts);
    // P2 (specific) and P3 (gear-level) beat the generic P4 at hook 30;
    // P5 (generic at hook 31) is mapped onto the bike.
    expect(res.map((p) => p.id)).toEqual(["P1", "P2", "P3", "P5"]);
  });

  it("degenerates to the part's own plans for a component part", async () => {
    await loadTypes([bikeType, wheelAsmType]);
    const wheel = part({ id: 5, what: 10 });
    const atts = attMap(att({ part_id: 5, gear: 100, hook: 30, what: 10 }));
    const P2 = plan({ id: "P2", part: 5, what: 10, hook: null, km: "100" });
    const res = plansForAssembly(wheel, planMap(P2), atts);
    expect(res.map((p) => p.id)).toEqual(["P2"]);
  });

  it("sorts subtype-hook plans by hook even when the type lists hooks reversed", async () => {
    await loadTypes([bikeType, { ...wheelAsmType, hooks: [31, 30] }]);
    const bike = part({ id: 100, what: 1 });
    const P1 = plan({ id: "P1", part: 100, what: 1, hook: null, km: "100" });
    const P4 = plan({ id: "P4", part: null, what: 10, hook: 31, km: "100" });
    const P3 = plan({ id: "P3", part: null, what: 10, hook: 30, km: "100" });
    // Walk order is P1, P4 (hook 31 first), P3 (hook 30); the sorted result
    // puts hook 30 before hook 31, which the old buggy comparator did not.
    const res = plansForAssembly(bike, planMap(P1, P4, P3), {});
    expect(res.map((p) => p.id)).toEqual(["P1", "P3", "P4"]);
  });
});

describe("isTemplate", () => {
  it("returns the part of the plan when it exists", () => {
    const p = plan({ id: "P1", part: 5 });
    expect(isTemplate(p, planMap(p))).toBe(5);
  });

  it("returns undefined when the plan is absent", () => {
    const p = plan({ id: "P1", part: 5 });
    expect(isTemplate(p, planMap())).toBeUndefined();
  });

  it("returns undefined for a plan without a part", () => {
    const p = plan({ id: "P1", part: null });
    expect(isTemplate(p, planMap(p))).toBeUndefined();
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
