import { get } from "svelte/store";
import { describe, expect, it } from "vitest";
import { by, filterValues, mapable, mapObject, type Map } from "./mapable";

type Item = {
  id: number;
  val: string;
};

describe("mapObject", () => {
  it("adds the object keyed by the given field", () => {
    const reduce = mapObject<Item>("id");
    const map = reduce({}, { id: 1, val: "a" });
    expect(map).toEqual({ "1": { id: 1, val: "a" } });
  });

  it("overwrites an existing entry with the same key", () => {
    const reduce = mapObject<Item>("id");
    let map: Map<Item> = { "1": { id: 1, val: "old" } };
    map = reduce(map, { id: 1, val: "new" });
    expect(map["1"].val).toBe("new");
  });

  it("deletes the entry when the del predicate matches", () => {
    const reduce = mapObject<Item>("id", (v) => v.val === "delete-me");
    let map: Map<Item> = { "1": { id: 1, val: "keep" } };
    map = reduce(map, { id: 2, val: "delete-me" });
    expect(map["2"]).toBeUndefined();
    expect(map["1"].val).toBe("keep");
  });
});

describe("mapable", () => {
  it("setMap replaces the whole map", () => {
    const store = mapable<Item>("id");
    store.setMap([
      { id: 1, val: "a" },
      { id: 2, val: "b" },
    ]);
    expect(get(store)).toEqual({
      "1": { id: 1, val: "a" },
      "2": { id: 2, val: "b" },
    });
    store.setMap([{ id: 3, val: "c" }]);
    expect(get(store)).toEqual({ "3": { id: 3, val: "c" } });
  });

  it("applies the prep function to each entry", () => {
    const store = mapable<Item>("id", (v) => ({ id: v.id, val: v.val + "!" }));
    store.setMap([{ id: 1, val: "a" }]);
    expect(get(store)["1"].val).toBe("a!");
  });

  it("skips items matching the del predicate on setMap", () => {
    const store = mapable<Item>("id", undefined, (v) => v.val === "gone");
    store.setMap([
      { id: 1, val: "stay" },
      { id: 2, val: "gone" },
    ]);
    expect(get(store)).toEqual({ "1": { id: 1, val: "stay" } });
  });

  it("removes a pre-existing entry matching the del predicate on updateMap", () => {
    const store = mapable<Item>("id", undefined, (v) => v.val === "gone");
    store.setMap([{ id: 1, val: "old" }]);
    store.updateMap([{ id: 1, val: "gone" }]);
    expect(get(store)).toEqual({});
  });

  it("updateMap merges into the existing map", () => {
    const store = mapable<Item>("id");
    store.setMap([{ id: 1, val: "a" }]);
    store.updateMap([{ id: 2, val: "b" }]);
    expect(get(store)).toEqual({
      "1": { id: 1, val: "a" },
      "2": { id: 2, val: "b" },
    });
    store.updateMap([{ id: 1, val: "a2" }]);
    expect(get(store)["1"].val).toBe("a2");
  });

  it("deleteItem removes a single entry", () => {
    const store = mapable<Item>("id");
    store.setMap([
      { id: 1, val: "a" },
      { id: 2, val: "b" },
    ]);
    store.deleteItem(1);
    expect(get(store)).toEqual({ "2": { id: 2, val: "b" } });
  });

  it("deleteItem with a falsy id is a no-op", () => {
    const store = mapable<Item>("id");
    store.setMap([{ id: 1, val: "a" }]);
    store.deleteItem(undefined);
    expect(get(store)).toEqual({ "1": { id: 1, val: "a" } });
  });
});

describe("filterValues", () => {
  const map: Map<Item> = {
    "1": { id: 1, val: "a" },
    "2": { id: 2, val: "b" },
    "3": { id: 3, val: "c" },
  };

  it("returns values matching the predicate", () => {
    expect(filterValues(map, (v) => v.id > 1)).toEqual([
      { id: 2, val: "b" },
      { id: 3, val: "c" },
    ]);
  });

  it("returns an empty array when nothing matches", () => {
    expect(filterValues(map, (v) => v.id > 100)).toEqual([]);
  });
});

describe("by", () => {
  const items: Item[] = [
    { id: 3, val: "c" },
    { id: 1, val: "a" },
    { id: 2, val: "b" },
  ];

  it("sorts descending by default (largest first)", () => {
    const sorted = [...items].sort(by<Item>("id"));
    expect(sorted.map((i) => i.id)).toEqual([3, 2, 1]);
  });

  it("sorts ascending when asc is true", () => {
    const sorted = [...items].sort(by<Item>("id", true));
    expect(sorted.map((i) => i.id)).toEqual([1, 2, 3]);
  });
});
