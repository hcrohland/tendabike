import { describe, expect, it, vi, beforeEach } from "vitest";
import { get } from "svelte/store";
import { myfetch, checkStatus, handleError, message } from "./store";
import { user } from "./user";
import { resp } from "../test/helpers";

describe("myfetch", () => {
  let fetchMock: ReturnType<typeof vi.fn>;

  beforeEach(() => {
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

  it("sets the message store on error", async () => {
    message.set({ active: false, message: "No message", status: "" });
    const text = "Something failed";
    await checkStatus(resp(text, 500, false, "Bad Gateway"))!.catch(() => {});
    expect(get(message)).toEqual({
      active: true,
      status: "Bad Gateway",
      message: text,
    });
  });
});

describe("checkStatus 401", () => {
  it("clears user, sets message, and rejects when user was set", async () => {
    user.set({
      id: 1,
      firstname: "A",
      name: "B",
      avatar: undefined,
      is_admin: false,
      onboarding_status: "pending",
    });
    message.set({ active: false, message: "No message", status: "" });
    await expect(
      checkStatus(resp("Unauthorized", 401, false, "")),
    ).rejects.toBe("Unauthorized");
    expect(get(user)).toBeUndefined();
    expect(get(message).active).toBe(true);
  });

  it("resolves undefined and does not set message when user was undefined", async () => {
    user.set(undefined);
    message.set({ active: false, message: "No message", status: "" });
    const result = await checkStatus(resp("Unauthorized", 401, false, ""));
    expect(result).toBeUndefined();
    expect(get(message).active).toBe(false);
  });
});

describe("handleError", () => {
  it("activates the message store", () => {
    message.set({ active: false, message: "No message", status: "" });
    handleError(new Error("boom"));
    const m = get(message);
    expect(m.active).toBe(true);
    expect(m.message).toBe("boom");
  });

  it("does not overwrite an already active message", () => {
    message.set({ active: true, message: "first", status: "" });
    handleError(new Error("second"));
    const m = get(message);
    expect(m.active).toBe(true);
    expect(m.message).toBe("first");
  });
});
