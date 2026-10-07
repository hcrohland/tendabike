import { describe, expect, it, vi, beforeEach } from "vitest";
import {
  initData,
  refresh,
  setSummary,
  updateSummary,
  getUser,
  setUser,
  users,
} from "./user";
import { parts } from "./part";
import { partNotes, PartNote } from "./partnote";
import { attachments, Attachment } from "./attachment";
import { activities } from "./activity";
import { services } from "./service";
import { usages } from "./usage";
import { plans } from "./serviceplan";
import { shops } from "./shop";
import { stateValues } from "./mapable.svelte";
import { resp, summary, summaryContent } from "../test/helpers";

const eva = { id: 2, firstname: "Eva", name: "Eva Example", avatar: undefined };

/// Seed all nine collections with one row each: the content Summary for the
/// seven collections it populates, plus direct rows for the two it leaves
/// empty.
function seedContent() {
  setSummary(summaryContent());
  partNotes.setMap([
    new PartNote({
      id: 7,
      part: 1,
      name: "Note",
      created: "2024-01-01T00:00:00Z",
    }),
  ]);
  attachments.setMap([
    new Attachment({
      part_id: 1,
      attached: "2024-01-01T00:00:00Z",
      detached: "2099-01-01T00:00:00Z",
      gear: 5,
      hook: 2,
      what: 10,
      name: "Tire",
    }),
  ]);
}

describe("initData", () => {
  let fetchMock: ReturnType<typeof vi.fn>;

  beforeEach(() => {
    setUser(undefined);
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
      .mockResolvedValueOnce(resp(summaryContent()));
    await initData();
    expect(fetchMock.mock.calls[0][0]).toBe("/api/user");
    expect(fetchMock.mock.calls[1][0]).toBe("/api/user/summary");
    expect(getUser()).toEqual(userData);
  });

  it("returns early without refresh when user is null", async () => {
    fetchMock.mockResolvedValueOnce(resp(null, 204, true, "No Content"));
    await initData();
    expect(fetchMock).toHaveBeenCalledTimes(1);
    expect(fetchMock.mock.calls[0][0]).toBe("/api/user");
    expect(getUser()).toBeUndefined();
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
    fetchMock.mockResolvedValue(resp(summaryContent()));
    await refresh();
    expect(fetchMock.mock.calls[0][0]).toBe("/api/user/summary");
    expect(parts[1]).toBeDefined();
  });

  it("appends shop query parameter", async () => {
    fetchMock.mockResolvedValue(resp(summaryContent()));
    await refresh(42);
    expect(fetchMock.mock.calls[0][0]).toBe("/api/user/summary?shop=42");
  });
});

describe("setSummary", () => {
  beforeEach(() => {
    parts.setMap([]);
    partNotes.setMap([]);
    attachments.setMap([]);
    activities.setMap([]);
    usages.setMap([]);
    services.setMap([]);
    plans.setMap([]);
    shops.setMap([]);
    users.setMap([]);
  });

  it("replaces all nine collections with the payload", () => {
    setSummary(summaryContent());
    expect(parts[1]).toBeDefined();
    expect(stateValues(partNotes)).toEqual([]);
    expect(stateValues(attachments)).toEqual([]);
    expect(activities[100]).toBeDefined();
    expect(usages["u1"]).toBeDefined();
    expect(services["S1"]).toBeDefined();
    expect(plans["PL1"]).toBeDefined();
    expect(shops[10]).toBeDefined();
    expect(users[1]).toBeDefined();
  });

  it("the payload wins where it carries rows, and collections it leaves empty are emptied", () => {
    seedContent();
    setSummary(summary({ users: { "2": eva } }));
    expect(users[2]).toBeDefined();
    expect(users[1]).toBeUndefined();
    expect(stateValues(parts)).toEqual([]);
    expect(stateValues(partNotes)).toEqual([]);
    expect(stateValues(attachments)).toEqual([]);
    expect(stateValues(activities)).toEqual([]);
    expect(stateValues(usages)).toEqual([]);
    expect(stateValues(services)).toEqual([]);
    expect(stateValues(plans)).toEqual([]);
    expect(stateValues(shops)).toEqual([]);
  });
});

describe("updateSummary", () => {
  let fetchMock: ReturnType<typeof vi.fn>;

  beforeEach(() => {
    parts.setMap([]);
    partNotes.setMap([]);
    attachments.setMap([]);
    activities.setMap([]);
    usages.setMap([]);
    services.setMap([]);
    plans.setMap([]);
    shops.setMap([]);
    users.setMap([]);
    fetchMock = vi.fn();
    vi.stubGlobal("fetch", fetchMock);
  });

  it("calls refresh() when data is undefined", () => {
    fetchMock.mockResolvedValue(resp(summaryContent()));
    updateSummary();
    expect(fetchMock).toHaveBeenCalledTimes(1);
    expect(fetchMock.mock.calls[0][0]).toBe("/api/user/summary");
  });

  it("merges all nine collections, including users", () => {
    updateSummary(summaryContent());
    expect(parts[1]).toBeDefined();
    expect(stateValues(partNotes)).toEqual([]);
    expect(stateValues(attachments)).toEqual([]);
    expect(activities[100]).toBeDefined();
    expect(usages["u1"]).toBeDefined();
    expect(services["S1"]).toBeDefined();
    expect(plans["PL1"]).toBeDefined();
    expect(shops[10]).toBeDefined();
    expect(users[1]).toBeDefined();
  });

  it("upserts user rows by id", () => {
    users.setMap([
      { id: 1, firstname: "Old", name: "Old Name", avatar: undefined },
      eva,
    ]);
    updateSummary(summaryContent());
    expect(stateValues(users)).toHaveLength(2);
    expect(users[1].name).toBe("Max Mustermann");
    expect(users[2].name).toBe("Eva Example");
  });

  it("keeps pre-existing rows the payload does not carry", () => {
    seedContent();
    updateSummary(summary({ users: { "2": eva } }));
    expect(users[1].name).toBe("Max Mustermann");
    expect(users[2]).toBeDefined();
    expect(stateValues(parts)).toHaveLength(1);
  });

  it("an empty payload removes nothing", () => {
    seedContent();
    updateSummary(summary());
    expect(stateValues(parts)).toHaveLength(1);
    expect(stateValues(partNotes)).toHaveLength(1);
    expect(stateValues(attachments)).toHaveLength(1);
    expect(stateValues(activities)).toHaveLength(1);
    expect(stateValues(usages)).toHaveLength(1);
    expect(stateValues(services)).toHaveLength(1);
    expect(stateValues(plans)).toHaveLength(1);
    expect(stateValues(shops)).toHaveLength(1);
    expect(stateValues(users)).toHaveLength(1);
  });

  it("a tombstone (null) deletes the row it names", () => {
    seedContent();
    updateSummary(summary({ parts: { "1": null } }));
    expect(parts[1]).toBeUndefined();
    expect(stateValues(parts)).toHaveLength(0);
  });

  it("a live attachment is upserted under the wire key, which is the frontend idx", () => {
    // The wire key is the frontend idx (part_id + "/" + attached ms), the same
    // key the client map uses for the row.
    const key = "1/" + new Date("2024-01-01T00:00:00Z").getTime();
    updateSummary(
      summary({
        attachments: {
          [key]: {
            part_id: 1,
            attached: "2024-01-01T00:00:00Z",
            detached: "2099-01-01T00:00:00Z",
            gear: 5,
            hook: 2,
            what: 10,
            name: "Tire",
          },
        },
      }),
    );
    expect(attachments[key]).toBeInstanceOf(Attachment);
  });

  it("a null attachment (tombstone) deletes the row it names", () => {
    seedContent();
    const key = "1/" + new Date("2024-01-01T00:00:00Z").getTime();
    updateSummary(summary({ attachments: { [key]: null } }));
    expect(stateValues(attachments)).toHaveLength(0);
  });

  it("an empty attachment (the detach flow) removes the row", () => {
    seedContent();
    updateSummary(
      summary({
        attachments: {
          ["1/" + new Date("2024-01-01T00:00:00Z").getTime()]: {
            part_id: 1,
            attached: "2024-01-01T00:00:00Z",
            detached: "2024-01-01T00:00:00Z",
            gear: 5,
            hook: 2,
            what: 10,
            name: "Tire",
          },
        },
      }),
    );
    expect(stateValues(attachments)).toHaveLength(0);
  });
});
