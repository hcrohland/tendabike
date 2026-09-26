import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  partForPlanGear,
  duesForPlans,
  gearsForPlan,
  alertCounts,
  plans,
  plansForPart,
  plansForAssembly,
  planCmp,
  isTemplate,
  localizeLimitKey,
  ServicePlan,
  type limit_keys,
} from "./serviceplan";
import { Part, parts } from "./part";
import { Attachment, attachments } from "./attachment";
import { Service, services } from "./service";
import { usages } from "./usage";
import { getTypes } from "./types";
import { maxDate } from "./store";
import { resp, usage } from "../test/helpers";

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
// that file would re-run its describes. The one exception is `usage`, shared
// from test/helpers (a non-test module, safe to import).

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

describe("partForPlanGear", () => {
  const part7 = part({ id: 7 });
  const part100 = part({ id: 100 });

  beforeEach(() => {
    parts.setMap([]);
    attachments.setMap([]);
  });

  // The collection's prep function rebuilds the part, so the door returns
  // a value-equal part, not the seeded instance.
  it("resolves the attached part at the hook", () => {
    parts.setMap([part7, part100]);
    attachments.setMap([att()]);
    const res = partForPlanGear(plan({ part: 100, what: 10, hook: 1 }));
    expect(res).toEqual(part7);
  });

  it("falls back to the gear part when nothing is attached", () => {
    parts.setMap([part7, part100]);
    const res = partForPlanGear(plan({ part: 100, what: 10, hook: 1 }));
    expect(res).toEqual(part100);
  });

  it("prefers the explicit gear over the plan's part", () => {
    parts.setMap([part7, part100]);
    const res = partForPlanGear(plan({ part: 100, what: 10, hook: 1 }), 7);
    expect(res).toEqual(part7);
  });

  it("returns null when there is no part", () => {
    expect(partForPlanGear(plan({ part: null }))).toBeNull();
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

  // The door, due_for and plan.services read the module state objects in
  // their bodies (issue #374); seed the world and reset it between tests.
  beforeEach(() => {
    services.setMap([]);
    usages.setMap([]);
  });

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
    services.setMap([service]);
    usages.setMap([u_prev, u_now]);
    expect(duesForPlans(p, [pln])).toEqual({
      rides: { due: 8, plan: 10, severity: "ok" },
      hours: { due: 1, plan: 5, severity: "ok" },
      km: { due: 61, plan: 100, severity: "ok" },
      climb: { due: 20, plan: 50, severity: "ok" },
      descend: { due: 5, plan: 20, severity: "ok" },
      kJ: { due: 100, plan: 1000, severity: "ok" },
    });
  });

  it("uses the full usage when there is no service", () => {
    const pln = plan({ what: 10, km: "100", rides: "10" });
    usages.setMap([u_prev, u_now]);
    expect(duesForPlans(p, [pln])).toEqual({
      rides: { due: 6, plan: 10, severity: "ok" },
      km: { due: 60, plan: 100, severity: "ok" },
    });
  });

  it("tracks the plan with the smallest due per limit", () => {
    const a = plan({ id: "A", what: 10, km: "200" });
    const b = plan({ id: "B", what: 10, km: "100" });
    usages.setMap([u_prev, u_now]);
    expect(duesForPlans(p, [a, b])).toEqual({
      km: { due: 60, plan: 100, severity: "ok" },
    });
  });

  // The per-limit severity verdict: the thresholds the badge used to
  // re-derive itself (issue #345), asserted through the door.
  const kmOnly = (distance: number) => {
    usages.setMap([usage("u_now", { distance })]);
    return duesForPlans(p, [plan({ what: 10, km: "100" })]).km!;
  };

  it("marks the verdict 'alert' when the remaining is negative", () => {
    expect(kmOnly(110000)).toEqual({ due: -10, plan: 100, severity: "alert" });
  });

  it("marks the verdict 'warn' when the remaining is within 5% of the limit", () => {
    expect(kmOnly(98000)).toEqual({ due: 2, plan: 100, severity: "warn" });
  });

  it("marks the verdict 'ok' exactly at the 5% edge", () => {
    expect(kmOnly(95000)).toEqual({ due: 5, plan: 100, severity: "ok" });
  });

  it("marks zero remaining 'warn' (the badge's behaviour)", () => {
    expect(kmOnly(100000)).toEqual({ due: 0, plan: 100, severity: "warn" });
  });

  it("marks the verdict 'ok' when the remaining is above the band", () => {
    expect(kmOnly(50000)).toEqual({ due: 50, plan: 100, severity: "ok" });
  });

  it("returns an empty object when nothing is due", () => {
    const a = plan({ what: 99, km: "100" });
    usages.setMap([u_prev, u_now]);
    expect(duesForPlans(p, [a])).toEqual({});
  });

  it("returns an empty object when the part type differs", () => {
    usages.setMap([u_prev, u_now]);
    expect(duesForPlans(part({ what: 99 }), [plan({ km: "100" })])).toEqual({});
  });

  it("returns an empty object for a null part", () => {
    usages.setMap([u_prev, u_now]);
    expect(duesForPlans(null, [plan({ km: "100" })])).toEqual({});
  });
});

describe("gearsForPlan", () => {
  // The door and its helper read the module state objects in their bodies
  // (issue #372); seed the world and reset it between tests.
  beforeEach(() => {
    parts.setMap([]);
    attachments.setMap([]);
    plans.setMap([]);
  });

  it("resolves a specific plan to its own part", () => {
    const p5 = part({ id: 5 });
    const pln = plan({ id: "P1", part: 5, what: 10, hook: null });
    parts.setMap([p5]);
    plans.setMap([pln]);
    expect(gearsForPlan(pln)).toEqual([p5]);
  });

  it("resolves a generic plan to all active parts of the type", async () => {
    await loadTypes([wheelType]);
    const p5 = part({ id: 5, what: 10 });
    const p7 = part({ id: 7, what: 10 });
    const G = plan({ id: "G", part: null, hook: 30, what: 10, km: "100" });
    parts.setMap([p5, p7]);
    plans.setMap([G]);
    // The door returns record order (the state proxy enumerates a
    // deleted-and-re-added key at its first-creation slot); the set of gears
    // is the contract, so compare the ids sorted.
    expect(
      gearsForPlan(G)
        .map((p) => p.id!)
        .sort((a, b) => a - b),
    ).toEqual([5, 7]);
  });

  it("excludes disposed parts", async () => {
    await loadTypes([wheelType]);
    const p5 = part({ id: 5, what: 10, disposed_at: "2024-01-01T00:00:00Z" });
    const p7 = part({ id: 7, what: 10 });
    const G = plan({ id: "G", part: null, hook: 30, what: 10, km: "100" });
    parts.setMap([p5, p7]);
    plans.setMap([G]);
    expect(gearsForPlan(G)).toEqual([p7]);
  });

  it("excludes parts covered by a dedicated plan", async () => {
    await loadTypes([wheelType]);
    const p5 = part({ id: 5, what: 10, usage: "u5" });
    const p7 = part({ id: 7, what: 10, usage: "u7" });
    const a = att({ part_id: 7, gear: 5, hook: 30, what: 10 });
    const G = plan({ id: "G", part: null, hook: 30, what: 10, km: "100" });
    const P7 = plan({ id: "P7", part: 7, hook: null, what: 10, km: "10" });
    parts.setMap([p5, p7]);
    attachments.setMap([a]);
    plans.setMap([G, P7]);
    expect(gearsForPlan(G)).toEqual([p7]);
  });
});

describe("alertCounts", () => {
  const p = part({ id: 5, what: 10, usage: "u_now" });

  // Seed the module state objects with the same world the arguments model:
  // partForPlanGear, gearsForPlan, plan.services and alert_for read them in
  // their bodies (issues #371/#372/#374).
  beforeEach(() => {
    parts.setMap([p]);
    attachments.setMap([]);
    plans.setMap([]);
    services.setMap([]);
    usages.setMap([]);
  });

  it("counts 'warn' when the due value is close to zero", () => {
    usages.setMap([usage("u_now", { distance: 97000 })]);
    expect(alertCounts([plan({ part: 5, what: 10, km: "100" })])).toEqual({
      warn: 1,
      alert: 0,
    });
  });

  it("counts 'alert' when the due value is negative", () => {
    usages.setMap([usage("u_now", { distance: 110000 })]);
    expect(alertCounts([plan({ part: 5, what: 10, km: "100" })])).toEqual({
      warn: 0,
      alert: 1,
    });
  });

  it("does not count a zero-remaining limit (the scan's zero guard)", () => {
    usages.setMap([usage("u_now", { distance: 100000 })]);
    expect(alertCounts([plan({ part: 5, what: 10, km: "100" })])).toEqual({
      warn: 0,
      alert: 0,
    });
  });

  it("counts nothing when the service is not due soon", () => {
    usages.setMap([usage("u_now", { distance: 50000 })]);
    expect(alertCounts([plan({ part: 5, what: 10, km: "100" })])).toEqual({
      warn: 0,
      alert: 0,
    });
  });

  it("counts nothing when the part does not match", () => {
    usages.setMap([usage("u_now", { distance: 97000 })]);
    expect(alertCounts([plan({ part: 5, what: 99, km: "100" })])).toEqual({
      warn: 0,
      alert: 0,
    });
  });

  it("does not count parts covered by a dedicated plan", async () => {
    await loadTypes([wheelType]);
    const p5 = part({ id: 5, what: 10, usage: "u5" });
    const p7 = part({ id: 7, what: 10, usage: "u7" });
    usages.setMap([
      usage("u5", { distance: 97000 }),
      usage("u7", { distance: 97000 }),
    ]);
    const a = att({ part_id: 7, gear: 5, hook: 30, what: 10 });
    const G = plan({ id: "G", part: null, hook: 30, what: 10, km: "100" });
    const P7 = plan({ id: "P7", part: 7, hook: null, what: 10, km: "10" });
    parts.setMap([p5, p7]);
    attachments.setMap([a]);
    plans.setMap([G, P7]);
    expect(alertCounts([G, P7])).toEqual({ warn: 1, alert: 1 });
  });
});

describe("plansForPart", () => {
  // The walk helpers read the module state objects in their bodies (issue
  // #373); seed the world and reset it between tests.
  beforeEach(() => {
    plans.setMap([]);
    attachments.setMap([]);
  });

  it("returns the part's own plans when it is not attached", () => {
    plans.setMap([
      plan({ id: "P1", part: 5, hook: null }),
      plan({ id: "P2", part: 5, hook: 1 }),
      plan({ id: "P3", part: 99, hook: null }),
    ]);
    const res = plansForPart(5);
    expect(res.map((p) => p.id)).toEqual(["P1"]);
  });

  it("returns specific and gear-level plans for an attached part", () => {
    plans.setMap([
      plan({ id: "P1", part: 7, hook: null }),
      plan({ id: "P2", part: 100, hook: 1, what: 10 }),
      plan({ id: "P3", part: 99, hook: 1, what: 10 }),
    ]);
    attachments.setMap([att()]);
    const res = plansForPart(7);
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
    plans.setMap([
      generic,
      plan({ id: "P2", part: null, hook: 2, what: 10 }),
      plan({ id: "P3", part: null, hook: 1, what: 20 }),
      plan({ id: "P4", part: 99, hook: 1, what: 10 }),
    ]);
    attachments.setMap([att()]);
    const res = plansForPart(7);
    expect(res).toHaveLength(1);
    expect(res[0].part).toBe(7);
    expect(res[0].what).toBe(10);
    expect(res[0].hook).toBe(1);
    expect(res[0]).not.toBe(generic);
  });

  it("treats a not-yet-attached part as unattached at the given time", () => {
    plans.setMap([plan({ id: "P1", part: 5, hook: null })]);
    attachments.setMap([
      att({
        part_id: 5,
        attached: "2030-01-01T00:00:00Z",
        detached: maxDate.getTime(),
      }),
    ]);
    const res = plansForPart(5, new Date("2029-01-01T00:00:00Z"));
    expect(res.map((p) => p.id)).toEqual(["P1"]);
  });
});

describe("plansForAssembly", () => {
  // The walk helpers read the module state objects in their bodies (issue
  // #373); seed the world and reset it between tests.
  beforeEach(() => {
    plans.setMap([]);
    attachments.setMap([]);
  });

  it("returns the gear's own plans plus the plans of attached parts", async () => {
    await loadTypes([bikeType, wheelAsmType]);
    const bike = part({ id: 100, what: 1 });
    attachments.setMap([att({ part_id: 5, gear: 100, hook: 30, what: 10 })]);
    const P1 = plan({ id: "P1", part: 100, what: 1, hook: null, km: "100" });
    const P2 = plan({ id: "P2", part: 5, what: 10, hook: null, km: "100" });
    const P3 = plan({ id: "P3", part: 100, what: 10, hook: 30, km: "100" });
    const P4 = plan({ id: "P4", part: null, what: 10, hook: 30, km: "100" });
    const P5 = plan({ id: "P5", part: null, what: 10, hook: 31, km: "100" });
    plans.setMap([P1, P2, P3, P4, P5]);
    const res = plansForAssembly(bike);
    // P2 (specific) and P3 (gear-level) beat the generic P4 at hook 30;
    // P5 (generic at hook 31) is mapped onto the bike.
    expect(res.map((p) => p.id)).toEqual(["P1", "P2", "P3", "P5"]);
  });

  it("degenerates to the part's own plans for a component part", async () => {
    await loadTypes([bikeType, wheelAsmType]);
    const wheel = part({ id: 5, what: 10 });
    attachments.setMap([att({ part_id: 5, gear: 100, hook: 30, what: 10 })]);
    const P2 = plan({ id: "P2", part: 5, what: 10, hook: null, km: "100" });
    plans.setMap([P2]);
    const res = plansForAssembly(wheel);
    expect(res.map((p) => p.id)).toEqual(["P2"]);
  });

  it("walks subtype hooks in the type's listed order (unsorted)", async () => {
    await loadTypes([bikeType, { ...wheelAsmType, hooks: [31, 30] }]);
    const bike = part({ id: 100, what: 1 });
    const P1 = plan({ id: "P1", part: 100, what: 1, hook: null, km: "100" });
    const P4 = plan({ id: "P4", part: null, what: 10, hook: 31, km: "100" });
    const P3 = plan({ id: "P3", part: null, what: 10, hook: 30, km: "100" });
    // The walk visits hooks in the type's listed order: the bike's own plan
    // (P1), then hook 31 (P4), then hook 30 (P3). The door no longer sorts;
    // ordering is the caller's job (see planCmp).
    plans.setMap([P1, P4, P3]);
    const res = plansForAssembly(bike);
    expect(res.map((p) => p.id)).toEqual(["P1", "P4", "P3"]);
  });
});

describe("planCmp", () => {
  it("orders by type, then hook, then part, then id; the hook branch compares hook", () => {
    const P1 = plan({ id: "P1", part: 100, what: 1, hook: null });
    const P4 = plan({ id: "P4", part: null, what: 10, hook: 31 });
    const P3 = plan({ id: "P3", part: null, what: 10, hook: 30 });
    // P3 (hook 30) sorts before P4 (hook 31) within the same type even though
    // P4 is listed first; the old buggy comparator compared the type here.
    expect([P1, P4, P3].sort(planCmp).map((p) => p.id)).toEqual([
      "P1",
      "P3",
      "P4",
    ]);
  });

  it("orders by type, then part, then id", () => {
    const P1 = plan({ id: "P1", part: 5, what: 20, hook: null });
    const P2 = plan({ id: "P2", part: 5, what: 10, hook: null });
    const P3 = plan({ id: "P3", part: 5, what: 10, hook: null });
    expect([P1, P2, P3].sort(planCmp).map((p) => p.id)).toEqual([
      "P2",
      "P3",
      "P1",
    ]);
  });

  it("orders by hook, then part within a type", () => {
    const B = plan({ id: "B", part: 7, what: 10, hook: 1 });
    const C = plan({ id: "C", part: 100, what: 10, hook: 1 });
    const D = plan({ id: "D", part: 7, what: 10, hook: 30 });
    expect([C, D, B].sort(planCmp).map((p) => p.id)).toEqual(["B", "C", "D"]);
  });
});

describe("isTemplate", () => {
  beforeEach(() => {
    plans.setMap([]);
  });

  it("is false for a plan bound to a specific part", () => {
    const p = plan({ id: "P1", part: 5 });
    plans.setMap([p]);
    expect(isTemplate(p)).toBe(false);
  });

  it("is true when the plan is absent from the map", () => {
    const p = plan({ id: "P1", part: 5 });
    expect(isTemplate(p)).toBe(true);
  });

  it("is true for a plan without a part", () => {
    const p = plan({ id: "P1", part: null });
    plans.setMap([p]);
    expect(isTemplate(p)).toBe(true);
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
