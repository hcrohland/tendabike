import { describe, expect, it, vi, beforeEach } from "vitest";
import { Service, services } from "./service";
import { Part } from "./part";
import { Usage, usages } from "./usage";
import { fmtDate, get_days } from "./store";
import { type Map } from "./mapable.svelte";
import { resp, usage } from "../test/helpers";

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

function usageMap(...us: Usage[]): Map<Usage> {
  return Object.fromEntries(us.map((u) => [u.id, u])) as Map<Usage>;
}

// Test-only helpers, moved out of the Service class (issue #385); they stay
// pure over their local maps.
function getRow(
  svc: Service,
  depth: number,
  part: Part,
  usages: Map<Usage>,
  successor: Service | null,
) {
  let next;
  let time: Date;
  if (!successor) {
    next = part.usage;
    time = new Date();
  } else {
    next = successor.usage;
    time = successor.time;
  }
  // svc.usage is undefined for the period without a service
  // this period starts at time part.purchase and has an empty usage
  if (!svc.usage) svc.time = part.purchase;
  let usage = svc.usage ? usages[next].sub(usages[svc.usage]) : usages[next];

  // How many days passed
  let days = get_days(svc.time, time);
  return { depth, service: svc, days, usage };
}

function fmtTime(svc: Service, s: Map<Service>) {
  let res = fmtDate(svc.time);
  // get_successor() takes no argument after the conversion (it reads the
  // module state), so the lookup is inlined to keep this helper pure over
  // its local map.
  let successor = svc.successor ? s[svc.successor] : null;
  if (successor) res = res + " - " + fmtDate(successor.time);
  return res;
}

describe("Service.get_successor", () => {
  // The door reads the module state in its body (issue #385); seed the
  // world and reset it between tests.
  beforeEach(() => {
    services.setMap([]);
  });

  it("returns null when there is no successor", () => {
    expect(svc({ successor: null }).get_successor()).toBeNull();
  });

  it("returns the successor service when it is present", () => {
    const s1 = svc({ id: "S1", successor: "S2" });
    const s2 = svc({ id: "S2" });
    services.setMap([s1, s2]);
    // The collection's prep function rebuilds the service, so the door
    // returns a value-equal service, not the seeded instance.
    expect(s1.get_successor()).toEqual(s2);
  });

  it("returns null when the successor is missing from the map", () => {
    expect(svc({ successor: "MISSING" }).get_successor()).toBeNull();
  });
});

describe("Service.history", () => {
  // The door reads the module state in its body (issue #385); seed the
  // world and reset it between tests.
  beforeEach(() => {
    services.setMap([]);
  });

  it("walks back through predecessors to the oldest service", () => {
    const s2 = svc({ id: "S2", successor: null });
    const s1 = svc({ id: "S1", successor: "S2" });
    const s0 = svc({ id: "S0", successor: "S1" });
    services.setMap([s0, s1, s2]);
    const hist = s2.history(0);
    expect(hist.map((h) => h.service?.id ?? null)).toEqual(["S1", "S0", null]);
    expect(hist.map((h) => h.successor.id)).toEqual(["S2", "S1", "S0"]);
  });

  it("returns a single placeholder entry when there is no predecessor", () => {
    const s = svc({ id: "S5", successor: null });
    services.setMap([s]);
    const hist = s.history(3);
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

    const row = getRow(service, 1, p, usageMap(u_prev, u_now), successor);

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

    const row = getRow(service, 0, p, usageMap(u_now), null);

    expect(row.usage).toBe(u_now);
    expect(service.time).toBe(p.purchase);
    expect(typeof row.days).toBe("number");
  });
});

describe("Service.fmtTime", () => {
  it("shows a single date without a successor", () => {
    const s = svc({ time: "2023-05-01T00:00:00Z" });
    expect(fmtTime(s, {})).toBe(fmtDate(new Date("2023-05-01T00:00:00Z")));
  });

  it("shows a date range when there is a successor", () => {
    const s1 = svc({ id: "S1", time: "2023-05-01T00:00:00Z", successor: "S2" });
    const s2 = svc({ id: "S2", time: "2024-05-01T00:00:00Z" });
    expect(fmtTime(s1, svcMap(s1, s2))).toBe(
      fmtDate(new Date("2023-05-01T00:00:00Z")) +
        " - " +
        fmtDate(new Date("2024-05-01T00:00:00Z")),
    );
  });
});

describe("Service CRUD", () => {
  let fetchMock: ReturnType<typeof vi.fn>;

  beforeEach(() => {
    services.setMap([]);
    usages.setMap([]);
    fetchMock = vi.fn();
    vi.stubGlobal("fetch", fetchMock);
  });

  const summary = () => ({
    parts: [],
    part_notes: [],
    attachments: [],
    activities: [],
    services: [
      {
        id: "S1",
        part_id: 5,
        time: "2023-01-01T00:00:00Z",
        redone: "2023-01-01T00:00:00Z",
        name: "Test",
        notes: "",
        usage: "u1",
        successor: null,
        plans: [],
      },
    ],
    plans: [],
    usages: [],
    shops: [],
    users: [],
  });

  it("Service.create POSTs and calls updateSummary", async () => {
    const time = new Date("2024-01-01T00:00:00Z");
    const sum = summary();
    fetchMock.mockResolvedValueOnce(resp(sum));
    await Service.create(5, time, "Annual", "good", ["P1"]);
    expect(fetchMock).toHaveBeenCalledTimes(1);
    const [url, options] = fetchMock.mock.calls[0];
    expect(url).toBe("/api/service");
    expect(options.method).toBe("POST");
    expect(JSON.parse(options.body)).toMatchObject({
      part_id: 5,
      name: "Annual",
      notes: "good",
      plans: ["P1"],
    });
    expect(services["S1"]).toBeDefined();
  });

  it("Service.update PUTs and calls updateSummary", async () => {
    const s = svc({ id: "S1", name: "New Name" });
    fetchMock.mockResolvedValueOnce(resp(summary()));
    await s.update();
    expect(fetchMock).toHaveBeenCalledTimes(1);
    const [url, options] = fetchMock.mock.calls[0];
    expect(url).toBe("/api/service");
    expect(options.method).toBe("PUT");
  });

  it("Service.delete removes the service and usage from the store", async () => {
    services.updateMap([svc({ id: "S1", usage: "u1" })]);
    usages.updateMap([usage("u1")]);
    fetchMock.mockResolvedValueOnce(resp(summary()));
    const s = svc({ id: "S1", usage: "u1" });
    await s.delete();
    expect(fetchMock).toHaveBeenCalledTimes(1);
    const [url, options] = fetchMock.mock.calls[0];
    expect(url).toBe("/api/service/S1");
    expect(options.method).toBe("DELETE");
    expect(services["S1"]).toBeUndefined();
    expect(usages["u1"]).toBeUndefined();
  });

  it("Service.repeat POSTs to /api/service/redo", async () => {
    const s = svc({ id: "S1" });
    fetchMock.mockResolvedValueOnce(resp(summary()));
    await s.repeat();
    expect(fetchMock).toHaveBeenCalledTimes(1);
    const [url, options] = fetchMock.mock.calls[0];
    expect(url).toBe("/api/service/redo");
    expect(options.method).toBe("POST");
  });
});
