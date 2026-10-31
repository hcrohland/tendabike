import { describe, expect, it, vi, beforeEach } from "vitest";
import { myfetch, checkStatus, handleError } from "./utils";
import { getUser, setUser } from "./user";
import { toastsDoor, clearToasts } from "./notification.svelte";
import { Shop, setShop } from "./shop";
import { resp } from "../test/helpers";

beforeEach(() => {
  clearToasts();
});

describe("myfetch", () => {
  let fetchMock: ReturnType<typeof vi.fn>;

  beforeEach(() => {
    clearToasts();
    fetchMock = vi.fn();
    vi.stubGlobal("fetch", fetchMock);
  });

  it("calls fetch without options for GET", async () => {
    const body = { ok: true };
    fetchMock.mockResolvedValue(resp(body));
    const res = await myfetch("/api/test");
    expect(fetchMock).toHaveBeenCalledTimes(1);
    expect(fetchMock.mock.calls[0][0]).toBe("/api/test");
    expect(fetchMock.mock.calls[0][1]).toBeUndefined();
    expect(res).toBe(body);
  });

  it("sends method, credentials, headers and JSON body for POST", async () => {
    const body = { id: 1 };
    fetchMock.mockResolvedValue(resp(body));
    const data = { name: "test" };
    await myfetch("/api/part", "POST", data);
    const [url, option] = fetchMock.mock.calls[0];
    expect(url).toBe("/api/part");
    expect(option.method).toBe("POST");
    expect(option.credentials).toBe("include");
    expect(option.headers).toEqual({ "Content-Type": "application/json" });
    expect(option.body).toBe(JSON.stringify(data));
  });

  it("uses the same option shape for PUT and DELETE", async () => {
    fetchMock.mockResolvedValue(resp(null, 204, true, ""));
    await myfetch("/api/part/1", "PUT", { x: 1 });
    let [url, option] = fetchMock.mock.calls[0];
    expect(url).toBe("/api/part/1");
    expect(option.method).toBe("PUT");
    expect(option.body).toBe(JSON.stringify({ x: 1 }));

    fetchMock.mockClear();
    fetchMock.mockResolvedValue(resp(null, 204, true, ""));
    await myfetch("/api/part/1", "DELETE");
    [url, option] = fetchMock.mock.calls[0];
    expect(url).toBe("/api/part/1");
    expect(option.method).toBe("DELETE");
  });

  it("resolves null for a 204 response", async () => {
    fetchMock.mockResolvedValue(resp(null, 204, true, ""));
    const res = await myfetch("/api/test");
    expect(res).toBeNull();
  });
});

describe("myfetch X-Active-Shop header (ADR-0009)", () => {
  let fetchMock: ReturnType<typeof vi.fn>;

  beforeEach(() => {
    setShop(undefined);
    fetchMock = vi.fn();
    vi.stubGlobal("fetch", fetchMock);
  });

  it("attaches the header from the tab's active shop on a GET", async () => {
    fetchMock.mockResolvedValue(resp({ ok: true }));
    setShop(new Shop({ id: 10 }));
    await myfetch("/api/user");
    const [url, option] = fetchMock.mock.calls[0];
    expect(url).toBe("/api/user");
    expect(option.headers).toEqual({ "X-Active-Shop": "10" });
  });

  it("attaches the header alongside the method's headers on a POST", async () => {
    fetchMock.mockResolvedValue(resp({ ok: true }));
    setShop(new Shop({ id: 10 }));
    await myfetch("/api/part", "POST", { name: "test" });
    const [url, option] = fetchMock.mock.calls[0];
    expect(url).toBe("/api/part");
    expect(option.headers).toEqual({
      "Content-Type": "application/json",
      "X-Active-Shop": "10",
    });
  });

  it("omits the header when the tab has no active shop", async () => {
    fetchMock.mockResolvedValue(resp({ ok: true }));
    await myfetch("/api/user");
    const [url, option] = fetchMock.mock.calls[0];
    expect(url).toBe("/api/user");
    expect(option).toBeUndefined();
  });
});

describe("checkStatus", () => {
  it("resolves with parsed JSON for 200", async () => {
    const body = { hello: "world" };
    const result = await checkStatus(resp(body));
    expect(result).toBe(body);
  });

  it("resolves null for 204", async () => {
    const result = await checkStatus(resp(null, 204, true, ""));
    expect(result).toBeNull();
  });

  it("rejects with the response text for non-ok status", async () => {
    const text = "Something failed";
    await expect(
      checkStatus(resp(text, 500, false, "Internal Server Error")),
    ).rejects.toBe(text);
  });

  it("does not queue a toast on error (handleError is the sole toast creator)", async () => {
    const text = "Something failed";
    await checkStatus(resp(text, 500, false, "Bad Gateway"))!.catch(() => {});
    const toasts = toastsDoor();
    expect(toasts).toHaveLength(0);
  });
});

describe("checkStatus 401", () => {
  it("clears user and rejects when user was set", async () => {
    setUser({
      id: 1,
      firstname: "A",
      name: "B",
      avatar: undefined,
      is_admin: false,
      onboarding_status: "pending",
    });
    await expect(
      checkStatus(resp("Unauthorized", 401, false, "")),
    ).rejects.toBe("Unauthorized");
    expect(getUser()).toBeUndefined();
  });

  it("resolves undefined when user was undefined", async () => {
    setUser(undefined);
    const result = await checkStatus(resp("Unauthorized", 401, false, ""));
    expect(result).toBeUndefined();
  });
});

describe("handleError", () => {
  it("queues an ephemeral toast", () => {
    handleError(new Error("boom"));
    const toasts = toastsDoor();
    expect(toasts).toHaveLength(1);
    expect(toasts[0].message).toBe("boom");
  });

  it("queues multiple concurrent errors", () => {
    handleError(new Error("first"));
    handleError(new Error("second"));
    const toasts = toastsDoor();
    expect(toasts).toHaveLength(2);
    expect(toasts[0].message).toBe("second");
    expect(toasts[1].message).toBe("first");
  });
});
