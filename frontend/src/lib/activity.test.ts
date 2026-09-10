import { describe, expect, it, vi, beforeEach } from "vitest";
import { Activity, activities } from "./activity";
import { Part } from "./part";
import { get } from "svelte/store";
import type { Map } from "./mapable";
import { resp } from "../test/helpers";

function actData(overrides: Partial<any> = {}): any {
  return {
    id: 100,
    user_id: 1,
    what: 301,
    name: "Morning Ride",
    start: "2024-05-01T08:00:00Z",
    gear: 5,
    climb: 100,
    descend: 80,
    distance: 25000,
    time: 3600,
    duration: 3600,
    energy: 2000,
    device_name: "Garmin",
    ...overrides,
  };
}

function summary(overrides: Partial<any> = {}): any {
  return {
    parts: [],
    attachments: [],
    activities: [],
    usages: [],
    services: [],
    plans: [],
    shops: [],
    users: [],
    ...overrides,
  };
}

function partData(overrides: Partial<any> = {}): any {
  return {
    id: 5,
    owner: 1,
    what: 1,
    name: "Bike A",
    vendor: "",
    model: "",
    purchase: "2023-01-01T00:00:00Z",
    last_used: "2024-01-01T00:00:00Z",
    disposed_at: null,
    usage: "u1",
    notes: "",
    shop: null,
    ...overrides,
  };
}

describe("Activity.update", () => {
  let fetchMock: ReturnType<typeof vi.fn>;

  beforeEach(() => {
    activities.setMap([]);
    fetchMock = vi.fn();
    vi.stubGlobal("fetch", fetchMock);
  });

  it("PUTs a copy of the activity to /api/activ/{id}", async () => {
    const updated = actData({ id: 100, name: "Updated Ride" });
    const sum = summary({ activities: [updated] });
    fetchMock.mockResolvedValue(resp(sum));
    const a = new Activity(actData({ id: 100 }));
    a.name = "Updated Ride";
    await a.update();
    const [url, option] = fetchMock.mock.calls[0];
    expect(url).toBe("/api/activ/100");
    expect(option.method).toBe("PUT");
    const body = JSON.parse(option.body);
    expect(body.name).toBe("Updated Ride");
    expect(body.id).toBe(100);
  });

  it("updates the activities map via updateSummary", async () => {
    const updated = actData({ id: 100, name: "New Name" });
    const sum = summary({ activities: [updated] });
    fetchMock.mockResolvedValue(resp(sum));
    const a = new Activity(actData({ id: 100 }));
    a.name = "New Name";
    await a.update();
    const map = get(activities);
    expect(map[100]).toBeInstanceOf(Activity);
    expect(map[100].name).toBe("New Name");
  });
});

describe("Activity.gearLink", () => {
  it("returns the part link when the gear part exists", () => {
    const a = new Activity(actData({ gear: 5 }));
    const part5 = new Part(partData({ id: 5, name: "Bike A" }));
    const partsMap = { 5: part5 } as Map<Part>;
    expect(a.gearLink(partsMap)).toBe(
      `<a href="/#/part/5" style="text-decoration:none" class="text-reset">Bike A</a>`,
    );
  });

  it("returns '-' when there is no gear", () => {
    const a = new Activity(actData({ gear: null }));
    expect(a.gearLink({})).toBe("-");
  });

  it("returns '-' when the gear part is not in the map", () => {
    const a = new Activity(actData({ gear: 99 }));
    expect(a.gearLink({})).toBe("-");
  });
});

describe("Activity.gearName", () => {
  it("returns the part name when the gear part exists", () => {
    const a = new Activity(actData({ gear: 5 }));
    const part5 = new Part(partData({ id: 5, name: "Bike A" }));
    const partsMap = { 5: part5 } as Map<Part>;
    expect(a.gearName(partsMap)).toBe("Bike A");
  });

  it("returns '-' when there is no gear", () => {
    const a = new Activity(actData({ gear: null }));
    expect(a.gearName({})).toBe("-");
  });

  it("returns '-' when the gear part is not in the map", () => {
    const a = new Activity(actData({ gear: 99 }));
    expect(a.gearName({})).toBe("-");
  });
});
