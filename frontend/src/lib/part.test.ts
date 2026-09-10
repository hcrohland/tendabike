import { describe, expect, it, vi, beforeEach } from "vitest";
import { Part, parts } from "./part";
import { get } from "svelte/store";
import { resp } from "../test/helpers";

function partData(overrides: Partial<any> = {}): any {
  return {
    id: 1,
    owner: 1,
    what: 10,
    name: "Wheel",
    vendor: "Velo",
    model: "X1",
    purchase: "2023-01-01T00:00:00Z",
    last_used: "2024-01-01T00:00:00Z",
    disposed_at: null,
    usage: "u1",
    notes: "",
    shop: null,
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

describe("Part CRUD", () => {
  let fetchMock: ReturnType<typeof vi.fn>;

  beforeEach(() => {
    parts.setMap([]);
    fetchMock = vi.fn();
    vi.stubGlobal("fetch", fetchMock);
  });

  describe("create", () => {
    it("POSTs the part and returns a new Part instance", async () => {
      const created = partData({ id: 42 });
      fetchMock.mockResolvedValue(resp(created));
      const p = new Part(partData({ id: undefined }));
      const result = (await p.create())!;
      const [url, option] = fetchMock.mock.calls[0];
      expect(url).toBe("/api/part");
      expect(option.method).toBe("POST");
      expect(result).toBeInstanceOf(Part);
      expect(result.id).toBe(42);
      expect(result.name).toBe("Wheel");
    });

    it("updates the parts map with the response", async () => {
      const created = partData({ id: 42 });
      fetchMock.mockResolvedValue(resp(created));
      const p = new Part(partData({ id: undefined }));
      await p.create();
      const map = get(parts);
      expect(map[42]).toBeInstanceOf(Part);
      expect(map[42].name).toBe("Wheel");
    });
  });

  describe("update", () => {
    it("PUTs the part to /api/part/{id}", async () => {
      const updated = partData({ id: 5, name: "New Wheel" });
      fetchMock.mockResolvedValue(resp(updated));
      const p = new Part(partData({ id: 5 }));
      p.name = "New Wheel";
      await p.update();
      const [url, option] = fetchMock.mock.calls[0];
      expect(url).toBe("/api/part/5");
      expect(option.method).toBe("PUT");
    });

    it("updates the parts map with the response", async () => {
      const updated = partData({ id: 5, name: "Changed" });
      fetchMock.mockResolvedValue(resp(updated));
      const p = new Part(partData({ id: 5, name: "Old" }));
      p.name = "Changed";
      await p.update();
      const map = get(parts);
      expect(map[5].name).toBe("Changed");
    });
  });

  describe("delete", () => {
    it("DELETEs /api/part/{id} and removes from the parts map", async () => {
      const existing = new Part(partData({ id: 5 }));
      parts.setMap([partData({ id: 5 })]);
      fetchMock.mockResolvedValue(resp(5));
      await existing.delete();
      const [url, option] = fetchMock.mock.calls[0];
      expect(url).toBe("/api/part/5");
      expect(option.method).toBe("DELETE");
      const map = get(parts);
      expect(map[5]).toBeUndefined();
    });
  });

  describe("attach", () => {
    it("POSTs to /api/part/attach with event data", async () => {
      fetchMock.mockResolvedValue(resp(summary()));
      const p = new Part(partData({ id: 5 }));
      const date = new Date("2024-01-01T00:00:00Z");
      await p.attach(date, true, 1, 2);
      const [url, option] = fetchMock.mock.calls[0];
      expect(url).toBe("/api/part/attach");
      expect(option.method).toBe("POST");
      const body = JSON.parse(option.body);
      expect(body.part_id).toBe(5);
      expect(body.all).toBe(true);
      expect(body.gear).toBe(1);
      expect(body.hook).toBe(2);
    });

    it("calls updateSummary with the response", async () => {
      const data = partData({ id: 5 });
      const sum = summary({ parts: [data] });
      fetchMock.mockResolvedValue(resp(sum));
      const p = new Part(partData({ id: 5 }));
      await p.attach(new Date(), true, 1, 2);
      const map = get(parts);
      expect(map[5]).toBeInstanceOf(Part);
    });
  });

  describe("detach", () => {
    it("POSTs to /api/part/detach with event data", async () => {
      fetchMock.mockResolvedValue(resp(summary()));
      const p = new Part(partData({ id: 5 }));
      const date = new Date("2024-06-01T00:00:00Z");
      await p.detach(date, false);
      const [url, option] = fetchMock.mock.calls[0];
      expect(url).toBe("/api/part/detach");
      expect(option.method).toBe("POST");
      const body = JSON.parse(option.body);
      expect(body.part_id).toBe(5);
      expect(body.all).toBe(false);
      expect(body.gear).toBe(0);
      expect(body.hook).toBe(0);
    });
  });

  describe("dispose", () => {
    it("POSTs to /api/part/dispose with event data", async () => {
      fetchMock.mockResolvedValue(resp(summary()));
      const p = new Part(partData({ id: 5 }));
      const date = new Date("2024-06-01T00:00:00Z");
      await p.dispose(date, true);
      const [url, option] = fetchMock.mock.calls[0];
      expect(url).toBe("/api/part/dispose");
      expect(option.method).toBe("POST");
      const body = JSON.parse(option.body);
      expect(body.part_id).toBe(5);
      expect(body.all).toBe(true);
    });
  });

  describe("recover", () => {
    it("POSTs to /api/part/recover", async () => {
      fetchMock.mockResolvedValue(resp(summary()));
      const p = new Part(partData({ id: 5 }));
      await p.recover(true);
      const [url, option] = fetchMock.mock.calls[0];
      expect(url).toBe("/api/part/recover");
      expect(option.method).toBe("POST");
      const body = JSON.parse(option.body);
      expect(body.part_id).toBe(5);
      expect(body.all).toBe(true);
    });
  });
});
