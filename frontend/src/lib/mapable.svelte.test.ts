import { describe, expect, it } from "vitest";
import { mapableState } from "./mapable.svelte";

type Item = {
  id: number;
  val: string;
};

// Op semantics of the state-object factory: the same contract as mapable()
// (mapable.test.ts), observed on the state record directly instead of
// through get(store).
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
