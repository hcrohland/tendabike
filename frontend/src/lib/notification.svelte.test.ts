import { beforeEach, describe, expect, it, vi } from "vitest";
import { deleteNotification } from "./notification.svelte";
import { resp } from "../test/helpers";

describe("notification delete door", () => {
  let fetchMock: ReturnType<typeof vi.fn>;

  beforeEach(() => {
    fetchMock = vi.fn();
    vi.stubGlobal("fetch", fetchMock);
  });

  it("DELETEs /api/notification/{id}; the tombstone arrives via the stream", async () => {
    fetchMock.mockResolvedValue(resp(null, 204, true, "No Content"));
    await deleteNotification("n1");
    const [url, option] = fetchMock.mock.calls[0];
    expect(url).toBe("/api/notification/n1");
    expect(option.method).toBe("DELETE");
  });
});
