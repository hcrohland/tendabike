import { describe, expect, it } from "vitest";
import { Service } from "./service";
import { Part } from "./part";
import { Usage } from "./usage";
import { fmtDate } from "./store";
import { type Map } from "./mapable";

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

describe("Service.get_successor", () => {
  it("returns null when there is no successor", () => {
    expect(svc({ successor: null }).get_successor({})).toBeNull();
  });

  it("returns the successor service when it is present", () => {
    const s1 = svc({ id: "S1", successor: "S2" });
    const s2 = svc({ id: "S2" });
    expect(s1.get_successor(svcMap(s1, s2))).toBe(s2);
  });

  it("returns null when the successor is missing from the map", () => {
    expect(svc({ successor: "MISSING" }).get_successor({})).toBeNull();
  });
});

describe("Service.history", () => {
  it("walks back through predecessors to the oldest service", () => {
    const s2 = svc({ id: "S2", successor: null });
    const s1 = svc({ id: "S1", successor: "S2" });
    const s0 = svc({ id: "S0", successor: "S1" });
    const hist = s2.history(0, svcMap(s0, s1, s2));
    expect(hist.map((h) => h.service?.id ?? null)).toEqual(["S1", "S0", null]);
    expect(hist.map((h) => h.successor.id)).toEqual(["S2", "S1", "S0"]);
  });

  it("returns a single placeholder entry when there is no predecessor", () => {
    const s = svc({ id: "S5", successor: null });
    const hist = s.history(3, svcMap(s));
    expect(hist.length).toBe(1);
    expect(hist[0].depth).toBe(2);
    expect(hist[0].service).toBeUndefined();
    expect(hist[0].successor).toBe(s);
  });
});

describe("Service.get_row", () => {
  it("computes the usage delta and days between two services", () => {
    const p = part({ id: 5, usage: "u_now" });
    const u_prev = usage("u_prev", { count: 2, distance: 1000 });
    const u_now = usage("u_now", { count: 4, distance: 40000 });
    const successor = svc({
      id: "S2",
      usage: "u_now",
      time: "2024-01-01T00:00:00Z",
    });
    const service = svc({
      id: "S1",
      usage: "u_prev",
      time: "2023-01-01T00:00:00Z",
    });

    const row = service.get_row(1, p, usageMap(u_prev, u_now), successor);

    expect(row.service).toBe(service);
    expect(row.depth).toBe(1);
    expect(row.usage).toBeInstanceOf(Usage);
    expect(row.usage.count).toBe(2);
    expect(row.usage.distance).toBe(39000);
    expect(row.days).toBe(365);
  });

  it("uses the full usage and pins time to purchase without a successor", () => {
    const p = part({ id: 5, usage: "u_now", purchase: "2023-01-01T00:00:00Z" });
    const u_now = usage("u_now", { count: 4, distance: 40000 });
    const service = svc({
      id: "S9",
      usage: undefined,
      time: "2020-01-01T00:00:00Z",
    });

    const row = service.get_row(0, p, usageMap(u_now), null);

    expect(row.usage).toBe(u_now);
    expect(service.time).toBe(p.purchase);
    expect(typeof row.days).toBe("number");
  });
});

describe("Service.fmtTime", () => {
  it("shows a single date without a successor", () => {
    const s = svc({ time: "2023-05-01T00:00:00Z" });
    expect(s.fmtTime({})).toBe(fmtDate(new Date("2023-05-01T00:00:00Z")));
  });

  it("shows a date range when there is a successor", () => {
    const s1 = svc({ id: "S1", time: "2023-05-01T00:00:00Z", successor: "S2" });
    const s2 = svc({ id: "S2", time: "2024-05-01T00:00:00Z" });
    expect(s1.fmtTime(svcMap(s1, s2))).toBe(
      fmtDate(new Date("2023-05-01T00:00:00Z")) +
        " - " +
        fmtDate(new Date("2024-05-01T00:00:00Z")),
    );
  });
});
