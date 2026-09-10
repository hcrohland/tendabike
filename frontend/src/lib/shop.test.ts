import { describe, expect, it, vi, beforeEach } from "vitest";
import { Shop, shops } from "./shop";
import { Part } from "./part";
import { get } from "svelte/store";
import { resp } from "../test/helpers";

function shopData(overrides: Partial<any> = {}): any {
  return {
    id: 10,
    owner: 1,
    name: "Velo Shop",
    description: "Local shop",
    auto_approve: true,
    created_at: "2023-01-01T00:00:00Z",
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
    what: 10,
    name: "Wheel",
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

describe("Shop CRUD", () => {
  let fetchMock: ReturnType<typeof vi.fn>;

  beforeEach(() => {
    shops.setMap([]);
    fetchMock = vi.fn();
    vi.stubGlobal("fetch", fetchMock);
  });

  describe("create", () => {
    it("POSTs a subset of fields to /api/shop", async () => {
      const created = shopData({ id: 10 });
      fetchMock.mockResolvedValue(resp(created));
      const s = new Shop(shopData({ id: undefined }));
      const result = await s.create();
      const [url, option] = fetchMock.mock.calls[0];
      expect(url).toBe("/api/shop");
      expect(option.method).toBe("POST");
      const body = JSON.parse(option.body);
      expect(body).toEqual({
        name: "Velo Shop",
        description: "Local shop",
        auto_approve: true,
      });
      expect(result!).toBeInstanceOf(Shop);
      expect((result as Shop).id).toBe(10);
    });

    it("updates the shops map with the response", async () => {
      const created = shopData({ id: 10 });
      fetchMock.mockResolvedValue(resp(created));
      const s = new Shop(shopData({ id: undefined }));
      await s.create();
      const map = get(shops);
      expect(map[10]).toBeInstanceOf(Shop);
      expect(map[10].name).toBe("Velo Shop");
    });
  });

  describe("update", () => {
    it("PUTs a subset of fields to /api/shop/{id}", async () => {
      const updated = shopData({ id: 10, name: "New Name" });
      fetchMock.mockResolvedValue(resp(updated));
      const s = new Shop(shopData({ id: 10 }));
      s.name = "New Name";
      await s.update();
      const [url, option] = fetchMock.mock.calls[0];
      expect(url).toBe("/api/shop/10");
      expect(option.method).toBe("PUT");
      const body = JSON.parse(option.body);
      expect(body).toEqual({
        name: "New Name",
        description: "Local shop",
        auto_approve: true,
      });
    });

    it("updates the shops map with the response", async () => {
      const updated = shopData({ id: 10, name: "Changed" });
      fetchMock.mockResolvedValue(resp(updated));
      const s = new Shop(shopData({ id: 10, name: "Old" }));
      s.name = "Changed";
      await s.update();
      const map = get(shops);
      expect(map[10].name).toBe("Changed");
    });
  });

  describe("delete", () => {
    it("DELETEs /api/shop/{id} and removes from the shops map", async () => {
      const s = new Shop(shopData({ id: 10 }));
      shops.setMap([shopData({ id: 10 })]);
      fetchMock.mockResolvedValue(resp(null, 204, true, "No Content"));
      await s.delete();
      const [url, option] = fetchMock.mock.calls[0];
      expect(url).toBe("/api/shop/10");
      expect(option.method).toBe("DELETE");
      const map = get(shops);
      expect(map[10]).toBeUndefined();
    });
  });

  describe("registerPart", () => {
    it("POSTs {part_id} to /api/shop/{shopid}/parts", async () => {
      fetchMock.mockResolvedValue(resp(summary()));
      const p = new Part(partData({ id: 5 }));
      await Shop.registerPart(p, 10);
      const [url, option] = fetchMock.mock.calls[0];
      expect(url).toBe("/api/shop/10/parts");
      expect(option.method).toBe("POST");
      const body = JSON.parse(option.body);
      expect(body).toEqual({ part_id: 5 });
    });
  });

  describe("unregisterPart", () => {
    it("DELETEs /api/shop/0/parts/{part_id}", async () => {
      fetchMock.mockResolvedValue(resp(summary()));
      const p = new Part(partData({ id: 5 }));
      await Shop.unregisterPart(p);
      const [url, option] = fetchMock.mock.calls[0];
      expect(url).toBe("/api/shop/0/parts/5");
      expect(option.method).toBe("DELETE");
    });
  });

  describe("getParts", () => {
    it("GETs /api/shop/{id}/parts and returns the array", async () => {
      fetchMock.mockResolvedValue(resp([5, 6, 7]));
      const s = new Shop(shopData({ id: 10 }));
      const result = await s.getParts();
      const [url, option] = fetchMock.mock.calls[0];
      expect(url).toBe("/api/shop/10/parts");
      expect(option.method).toBe("GET");
      expect(result).toEqual([5, 6, 7]);
    });
  });

  describe("requestSubscription", () => {
    it("POSTs {shop_id, message} to /api/shop/subscriptions", async () => {
      fetchMock.mockResolvedValue(resp("ok"));
      const s = new Shop(shopData({ id: 10 }));
      await s.requestSubscription("hello");
      const [url, option] = fetchMock.mock.calls[0];
      expect(url).toBe("/api/shop/subscriptions");
      expect(option.method).toBe("POST");
      const body = JSON.parse(option.body);
      expect(body).toEqual({ shop_id: 10, message: "hello" });
    });

    it("sends undefined message when omitted", async () => {
      fetchMock.mockResolvedValue(resp("ok"));
      const s = new Shop(shopData({ id: 10 }));
      await s.requestSubscription();
      const [, option] = fetchMock.mock.calls[0];
      const body = JSON.parse(option.body);
      expect(body.shop_id).toBe(10);
      expect(body.message).toBeUndefined();
    });
  });
});
