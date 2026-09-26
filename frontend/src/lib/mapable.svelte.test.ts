import { describe, expect, it } from "vitest";
import {
  type Map,
  by,
  filterValues,
  mapableState,
  stateValues,
} from "./mapable.svelte";

type Item = {
  id: number;
  val: string;
};

// Op semantics of the state-object factory, observed on the state record
// directly.
describe("mapableState", () => {
  it("setMap replaces the whole map", () => {
    const map = mapableState<Item>("id");
    map.setMap([
      { id: 1, val: "a" },
      { id: 2, val: "b" },
    ]);
    expect(map["1"]!.val).toBe("a");
    expect(map["2"]!.val).toBe("b");
    map.setMap([{ id: 3, val: "c" }]);
    expect(map["3"]!.val).toBe("c");
    expect(map["1"]).toBeUndefined();
    expect(map["2"]).toBeUndefined();
  });

  it("applies the prep function to each entry", () => {
    const map = mapableState<Item>("id", (v) => ({
      id: v.id,
      val: v.val + "!",
    }));
    map.setMap([{ id: 1, val: "a" }]);
    expect(map["1"]!.val).toBe("a!");
  });

  it("skips items matching the del predicate on setMap", () => {
    const map = mapableState<Item>("id", undefined, (v) => v.val === "gone");
    map.setMap([
      { id: 1, val: "stay" },
      { id: 2, val: "gone" },
    ]);
    expect(map["1"]!.val).toBe("stay");
    expect(map["2"]).toBeUndefined();
  });

  it("removes a pre-existing entry matching the del predicate on updateMap", () => {
    const map = mapableState<Item>("id", undefined, (v) => v.val === "gone");
    map.setMap([{ id: 1, val: "old" }]);
    map.updateMap([{ id: 1, val: "gone" }]);
    expect(map["1"]).toBeUndefined();
  });

  it("updateMap merges into the existing map", () => {
    const map = mapableState<Item>("id");
    map.setMap([{ id: 1, val: "a" }]);
    map.updateMap([{ id: 2, val: "b" }]);
    expect(map["1"]!.val).toBe("a");
    expect(map["2"]!.val).toBe("b");
    map.updateMap([{ id: 1, val: "a2" }]);
    expect(map["1"]!.val).toBe("a2");
  });

  it("deleteItem removes a single entry", () => {
    const map = mapableState<Item>("id");
    map.setMap([
      { id: 1, val: "a" },
      { id: 2, val: "b" },
    ]);
    map.deleteItem(1);
    expect(map["1"]).toBeUndefined();
    expect(map["2"]!.val).toBe("b");
  });

  it("deleteItem with a falsy id is a no-op", () => {
    const map = mapableState<Item>("id");
    map.setMap([{ id: 1, val: "a" }]);
    map.deleteItem(undefined);
    expect(map["1"]!.val).toBe("a");
  });
});

// Enumeration of the state record: the attached write operations are
// visible to Object.values, stateValues must skip them.
describe("stateValues", () => {
  it("returns the entity values of a state collection in record order", () => {
    const map = mapableState<Item>("id");
    map.setMap([
      { id: 1, val: "a" },
      { id: 2, val: "b" },
    ]);
    expect(stateValues(map)).toEqual([
      { id: 1, val: "a" },
      { id: 2, val: "b" },
    ]);
  });

  it("skips the attached write operations that Object.values surfaces", () => {
    const map = mapableState<Item>("id");
    map.setMap([{ id: 1, val: "a" }]);
    expect(Object.values(map).length).toBe(4);
    expect(stateValues(map)).toEqual([{ id: 1, val: "a" }]);
  });

  it("is a plain Object.values on a record without attached operations", () => {
    const plain: Map<Item> = { 1: { id: 1, val: "a" } };
    expect(stateValues(plain)).toEqual(Object.values(plain));
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
