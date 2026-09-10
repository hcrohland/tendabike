import { describe, expect, it, vi, beforeEach } from "vitest";
import {
  initData,
  refresh,
  setSummary,
  updateSummary,
  user,
  users,
} from "./user";
import { parts } from "./part";
import { activities } from "./activity";
import { services } from "./service";
import { usages } from "./usage";
import { attachments } from "./attachment";
import { plans } from "./serviceplan";
import { shops } from "./shop";
import { get } from "svelte/store";
import { resp } from "../test/helpers";

function summaryData(overrides: Partial<any> = {}): any {
  return {
    parts: [
      {
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
        notes: "",
        shop: null,
      },
    ],
    attachments: [],
    activities: [
      {
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
    ],
    usages: [
      {
        id: "u1",
        count: 1,
        climb: 0,
        descend: 0,
        distance: 1000,
        time: 3600,
        duration: 3600,
        energy: 100,
      },
    ],
    services: [
      {
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
    ],
    plans: [
      {
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
    ],
    shops: [
      {
        id: 10,
        owner: 1,
        name: "Shop",
        description: "",
        auto_approve: true,
        created_at: "2023-01-01T00:00:00Z",
      },
    ],
    users: [
      { id: 1, firstname: "Max", name: "Max Mustermann", avatar: undefined },
    ],
    ...overrides,
  };
}

describe("initData", () => {
  let fetchMock: ReturnType<typeof vi.fn>;

  beforeEach(() => {
    user.set(undefined);
    fetchMock = vi.fn();
    vi.stubGlobal("fetch", fetchMock);
  });

  it("sets user and calls refresh when user is returned", async () => {
    const userData = {
      id: 1,
      firstname: "Max",
      name: "Max Mustermann",
      avatar: undefined,
      is_admin: false,
      onboarding_status: "completed",
    };
    fetchMock
      .mockResolvedValueOnce(resp(userData))
      .mockResolvedValueOnce(resp(summaryData()));
    await initData();
    expect(fetchMock.mock.calls[0][0]).toBe("/api/user");
    expect(fetchMock.mock.calls[1][0]).toBe("/api/user/summary");
    expect(get(user)).toEqual(userData);
  });

  it("returns early without refresh when user is null", async () => {
    fetchMock.mockResolvedValueOnce(resp(null, 204, true, "No Content"));
    await initData();
    expect(fetchMock).toHaveBeenCalledTimes(1);
    expect(fetchMock.mock.calls[0][0]).toBe("/api/user");
    expect(get(user)).toBeUndefined();
  });
});

describe("refresh", () => {
  let fetchMock: ReturnType<typeof vi.fn>;

  beforeEach(() => {
    parts.setMap([]);
    activities.setMap([]);
    usages.setMap([]);
    services.setMap([]);
    plans.setMap([]);
    shops.setMap([]);
    users.setMap([]);
    attachments.setMap([]);
    fetchMock = vi.fn();
    vi.stubGlobal("fetch", fetchMock);
  });

  it("GETs /api/user/summary and calls setSummary", async () => {
    fetchMock.mockResolvedValue(resp(summaryData()));
    await refresh();
    expect(fetchMock.mock.calls[0][0]).toBe("/api/user/summary");
    const map = get(parts);
    expect(map[1]).toBeDefined();
  });

  it("appends shop query parameter", async () => {
    fetchMock.mockResolvedValue(resp(summaryData()));
    await refresh(42);
    expect(fetchMock.mock.calls[0][0]).toBe("/api/user/summary?shop=42");
  });
});

describe("setSummary", () => {
  beforeEach(() => {
    parts.setMap([]);
    activities.setMap([]);
    usages.setMap([]);
    services.setMap([]);
    plans.setMap([]);
    shops.setMap([]);
    users.setMap([]);
    attachments.setMap([]);
  });

  it("calls setMap on all 8 stores", () => {
    setSummary(summaryData() as any);
    expect(get(parts)[1]).toBeDefined();
    expect(get(activities)[100]).toBeDefined();
    expect(get(usages)["u1"]).toBeDefined();
    expect(get(services)["S1"]).toBeDefined();
    expect(get(plans)["PL1"]).toBeDefined();
    expect(get(shops)[10]).toBeDefined();
    expect(get(users)[1]).toBeDefined();
    expect(get(attachments)).toEqual({});
  });
});

describe("updateSummary", () => {
  let fetchMock: ReturnType<typeof vi.fn>;

  beforeEach(() => {
    parts.setMap([]);
    activities.setMap([]);
    usages.setMap([]);
    services.setMap([]);
    plans.setMap([]);
    shops.setMap([]);
    users.setMap([]);
    attachments.setMap([]);
    fetchMock = vi.fn();
    vi.stubGlobal("fetch", fetchMock);
  });

  it("calls refresh() when data is undefined", () => {
    fetchMock.mockResolvedValue(resp(summaryData()));
    updateSummary();
    expect(fetchMock).toHaveBeenCalledTimes(1);
    expect(fetchMock.mock.calls[0][0]).toBe("/api/user/summary");
  });

  it("calls updateMap on 7 stores (not users) when data is provided", () => {
    updateSummary(summaryData() as any);
    expect(get(parts)[1]).toBeDefined();
    expect(get(activities)[100]).toBeDefined();
    expect(get(usages)["u1"]).toBeDefined();
    expect(get(services)["S1"]).toBeDefined();
    expect(get(plans)["PL1"]).toBeDefined();
    expect(get(shops)[10]).toBeDefined();
    expect(get(attachments)).toEqual({});
    expect(get(users)).toEqual({});
  });
});
