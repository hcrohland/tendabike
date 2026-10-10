import { describe, expect, it, vi, beforeEach, afterEach } from "vitest";
import { startStream, stopStream } from "./stream";
import { parts } from "./part";
import { stateValues } from "./mapable.svelte";
import { setUser, getUser } from "./user";
import { message } from "./store";
import { setShop, Shop } from "./shop";
import { resp, summary, summaryContent } from "../test/helpers";

/// A stand-in for the native EventSource, which jsdom does not implement.
/// The stream module takes an injectable factory, so the suite drives the
/// lifecycle (open/message/error/close) directly.
class FakeES {
  static instances: FakeES[] = [];
  onopen: (() => void) | null = null;
  onmessage: ((e: { data: string }) => void) | null = null;
  onerror: (() => void) | null = null;
  closed = false;
  constructor(public url: string) {
    FakeES.instances.push(this);
  }
  close() {
    this.closed = true;
  }
  fireOpen() {
    this.onopen?.();
  }
  fireMessage(data: string) {
    this.onmessage?.({ data });
  }
  fireError() {
    this.onerror?.();
  }
}

const factory = (url: string) => new FakeES(url);

function theStream() {
  return FakeES.instances[FakeES.instances.length - 1];
}

describe("stream", () => {
  let fetchMock: ReturnType<typeof vi.fn>;

  beforeEach(() => {
    FakeES.instances = [];
    parts.setMap([]);
    setUser(undefined);
    setShop(undefined);
    message.active = false;
    fetchMock = vi.fn();
    fetchMock.mockResolvedValue(resp(summary()));
    vi.stubGlobal("fetch", fetchMock);
  });

  afterEach(() => {
    stopStream();
  });

  it("opens the stream on /api/user/stream", () => {
    startStream(() => {}, factory);
    expect(FakeES.instances).toHaveLength(1);
    expect(theStream().url).toBe("/api/user/stream");
  });

  it("on open, fetches the full summary and hydrates the maps", async () => {
    fetchMock.mockResolvedValue(resp(summaryContent()));
    const started = startStream(() => {}, factory);
    theStream().fireOpen();
    await started;
    expect(fetchMock).toHaveBeenCalledTimes(1);
    expect(fetchMock.mock.calls[0][0]).toBe("/api/user/summary");
    expect(parts[1]).toBeDefined();
  });

  it("passes each snapshot to onSnapshot, including the first", async () => {
    fetchMock.mockResolvedValue(resp(summaryContent()));
    let seen: Promise<void> | undefined;
    const started = startStream((p) => (seen = p), factory);
    theStream().fireOpen();
    await started;
    expect(seen).toBeInstanceOf(Promise);
    await seen!;
    expect(parts[1]).toBeDefined();
  });

  it("merges each stream frame via updateSummary", () => {
    startStream(() => {}, factory);
    theStream().fireOpen();
    theStream().fireMessage(
      JSON.stringify(
        summary({ parts: { "7": { id: 7, name: "Frame Part" } } }),
      ),
    );
    expect(parts[7]).toBeDefined();
    expect(parts[7].name).toBe("Frame Part");
  });

  it("a tombstone frame deletes the row it names", () => {
    parts.setMap([summaryContent().parts["1"]]);
    startStream(() => {}, factory);
    theStream().fireOpen();
    theStream().fireMessage(JSON.stringify(summary({ parts: { "1": null } })));
    expect(parts[1]).toBeUndefined();
    expect(stateValues(parts)).toHaveLength(0);
  });

  it("a reconnect re-runs the snapshot fetch", async () => {
    fetchMock.mockResolvedValue(resp(summaryContent()));
    startStream(() => {}, factory);
    theStream().fireOpen();
    await vi.waitFor(() => expect(fetchMock).toHaveBeenCalledTimes(1));
    theStream().fireError();
    theStream().fireOpen();
    await vi.waitFor(() => expect(fetchMock).toHaveBeenCalledTimes(2));
    expect(fetchMock.mock.calls[1][0]).toBe("/api/user/summary");
  });

  it("after 5 consecutive failed opens, probes liveness with GET /api/user", async () => {
    fetchMock.mockResolvedValue(resp({ id: 1 }));
    startStream(() => {}, factory).catch(() => {});
    for (let i = 0; i < 4; i++) theStream().fireError();
    expect(fetchMock).not.toHaveBeenCalledWith("/api/user", undefined);
    theStream().fireError();
    await vi.waitFor(() =>
      expect(fetchMock).toHaveBeenCalledWith("/api/user", undefined),
    );
  });

  it("a successful open resets the failed-open counter", async () => {
    fetchMock.mockResolvedValue(resp(summaryContent()));
    startStream(() => {}, factory).catch(() => {});
    for (let i = 0; i < 4; i++) theStream().fireError();
    theStream().fireOpen();
    await vi.waitFor(() => expect(fetchMock).toHaveBeenCalledTimes(1));
    for (let i = 0; i < 4; i++) theStream().fireError();
    await new Promise((r) => setTimeout(r, 10));
    expect(fetchMock).not.toHaveBeenCalledWith("/api/user", undefined);
  });

  it("a 401 probe clears the user and raises the banner", async () => {
    setUser({
      id: 1,
      firstname: "Max",
      name: "Mustermann",
      avatar: undefined,
      is_admin: false,
      onboarding_status: "completed",
    });
    fetchMock.mockResolvedValue(resp("Unauthorized", 401, false, ""));
    startStream(() => {}, factory).catch(() => {});
    for (let i = 0; i < 5; i++) theStream().fireError();
    await vi.waitFor(() => expect(getUser()).toBeUndefined());
    expect(message.active).toBe(true);
  });

  it("stopStream closes the source and ignores later events", () => {
    startStream(() => {}, factory);
    stopStream();
    expect(theStream().closed).toBe(true);
    theStream().fireMessage(
      JSON.stringify(summary({ parts: { "9": { id: 9, name: "Late" } } })),
    );
    expect(parts[9]).toBeUndefined();
  });

  it("the snapshot mirrors the manual refresh shop scope", async () => {
    fetchMock.mockResolvedValue(resp(summaryContent()));
    setShop(new Shop({ id: 10, name: "S" }));
    startStream(() => {}, factory);
    theStream().fireOpen();
    await vi.waitFor(() =>
      expect(fetchMock).toHaveBeenCalledWith(
        "/api/user/summary?shop=10",
        undefined,
      ),
    );
  });

  it("a failed first open rejects the start promise", async () => {
    const started = startStream(() => {}, factory);
    theStream().fireError();
    await expect(started).rejects.toThrow();
  });
});
