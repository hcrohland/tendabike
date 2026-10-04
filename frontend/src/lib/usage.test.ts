import { describe, expect, it } from "vitest";
import { Usage } from "./usage";
import { Activity } from "./activity";
import { usage } from "../test/helpers";

function actData(overrides: Partial<any> = {}): any {
  return {
    id: 100,
    user_id: 1,
    what: 301,
    name: "Morning Ride",
    start: "2024-05-01T08:00:00Z",
    gear: null,
    climb: 100,
    descend: 80,
    distance: 25000,
    time: 3600,
    duration: 3600,
    energy: 2000,
    device_name: "Garmin",
    ...overrides,
  };
}

describe("Usage.add", () => {
  it("accumulates a full activity into the usage", () => {
    const u = usage("u1", {
      count: 1,
      climb: 100,
      descend: 80,
      distance: 25000,
      time: 3600,
      duration: 3600,
      energy: 2000,
    });
    u.add(new Activity(actData()));
    expect(u.count).toBe(2);
    expect(u.climb).toBe(200);
    expect(u.descend).toBe(160);
    expect(u.distance).toBe(50000);
    expect(u.time).toBe(7200);
    expect(u.duration).toBe(7200);
    expect(u.energy).toBe(4000);
  });

  it("accumulates another usage field by field", () => {
    const u1 = usage("u1", {
      time: 1000,
      duration: 1000,
      distance: 5000,
      climb: 100,
      descend: 80,
      energy: 500,
      count: 1,
    });
    const u2 = usage("u2", {
      time: 2000,
      duration: 2000,
      distance: 15000,
      climb: 300,
      descend: 200,
      energy: 1500,
      count: 2,
    });
    u1.add(u2);
    expect(u1.time).toBe(3000);
    expect(u1.duration).toBe(3000);
    expect(u1.distance).toBe(20000);
    expect(u1.climb).toBe(400);
    expect(u1.descend).toBe(280);
    expect(u1.energy).toBe(2000);
    expect(u1.count).toBe(3);
  });

  it("mutates in place and keeps its id", () => {
    const u = usage("u1", { distance: 1000 });
    const before = u;
    u.add(usage("u2", { distance: 500 }));
    expect(u).toBe(before);
    expect(u.id).toBe("u1");
    expect(u.distance).toBe(1500);
  });

  it("counts an activity with a zero count as one ride", () => {
    const u = usage("u1", { count: 3 });
    u.add(
      new Activity(
        actData({
          climb: 0,
          descend: 0,
          distance: 0,
          time: 0,
          duration: 0,
          energy: 0,
        }),
      ),
    );
    expect(u.count).toBe(4);
  });

  it("adds one ride for a usage with a zero count", () => {
    const u = usage("u1", { count: 3 });
    u.add(usage("u2"));
    expect(u.count).toBe(4);
  });

  it("falls back to duration when the time is zero", () => {
    const u = usage("u1", { time: 100, duration: 100 });
    u.add(new Activity(actData({ time: 0, duration: 3600 })));
    expect(u.time).toBe(3700);
    expect(u.duration).toBe(3700);
  });

  it("falls back to time when the duration is zero", () => {
    const u = usage("u1", { time: 100, duration: 100 });
    u.add(new Activity(actData({ time: 3600, duration: 0 })));
    expect(u.time).toBe(3700);
    expect(u.duration).toBe(3700);
  });

  it("treats a missing time and duration as zero", () => {
    const u = usage("u1", { time: 100, duration: 100 });
    u.add(new Activity(actData({ time: null, duration: null })));
    expect(u.time).toBe(100);
    expect(u.duration).toBe(100);
  });

  it("falls back to climb when the descend is zero", () => {
    const u = usage("u1", { climb: 10, descend: 10 });
    u.add(new Activity(actData({ climb: 100, descend: 0 })));
    expect(u.climb).toBe(110);
    expect(u.descend).toBe(110);
  });

  it("adds zero for missing climb, distance, and energy", () => {
    const u = usage("u1", { climb: 10, distance: 1000, energy: 100 });
    u.add(new Activity(actData({ climb: null, distance: null, energy: null })));
    expect(u.climb).toBe(10);
    expect(u.distance).toBe(1000);
    expect(u.energy).toBe(100);
  });

  it("applies the same fallbacks to a usage rhs", () => {
    const u = usage("u1", {
      count: 3,
      time: 100,
      duration: 100,
      climb: 10,
      descend: 10,
    });
    u.add(
      usage("u2", {
        time: 0,
        duration: 3600,
        climb: 100,
        descend: 0,
      }),
    );
    expect(u.count).toBe(4);
    expect(u.time).toBe(3700);
    expect(u.duration).toBe(3700);
    expect(u.climb).toBe(110);
    expect(u.descend).toBe(110);
  });
});

describe("Usage.sub", () => {
  it("subtracts each field", () => {
    const u1 = usage("u1", {
      time: 3000,
      duration: 3000,
      distance: 20000,
      climb: 400,
      descend: 280,
      energy: 2000,
      count: 3,
    });
    const u2 = usage("u2", {
      time: 1000,
      duration: 1000,
      distance: 5000,
      climb: 100,
      descend: 80,
      energy: 500,
      count: 1,
    });
    const diff = u1.sub(u2);
    expect(diff).toBeInstanceOf(Usage);
    expect(diff.id).toBe("u1");
    expect(diff.time).toBe(2000);
    expect(diff.duration).toBe(2000);
    expect(diff.distance).toBe(15000);
    expect(diff.climb).toBe(300);
    expect(diff.descend).toBe(200);
    expect(diff.energy).toBe(1500);
    expect(diff.count).toBe(2);
  });

  it("returns a new usage without mutating the receiver", () => {
    const u1 = usage("u1", { time: 3000, distance: 20000, count: 3 });
    const u2 = usage("u2", { time: 1000, distance: 5000, count: 1 });
    const diff = u1.sub(u2);
    expect(diff).not.toBe(u1);
    expect(u1.time).toBe(3000);
    expect(u1.distance).toBe(20000);
    expect(u1.count).toBe(3);
  });

  it("returns a copy of itself with the default empty rhs", () => {
    const u = usage("u1", {
      time: 3000,
      duration: 3000,
      distance: 20000,
      climb: 400,
      descend: 280,
      energy: 2000,
      count: 3,
    });
    const copy = u.sub();
    expect(copy).not.toBe(u);
    expect(copy.id).toBe("u1");
    expect(copy.time).toBe(3000);
    expect(copy.duration).toBe(3000);
    expect(copy.distance).toBe(20000);
    expect(copy.climb).toBe(400);
    expect(copy.descend).toBe(280);
    expect(copy.energy).toBe(2000);
    expect(copy.count).toBe(3);
  });

  it("returns a copy of itself with an explicit empty usage", () => {
    const u = usage("u1", { time: 3000, distance: 20000, count: 3 });
    const copy = u.sub(new Usage());
    expect(copy).not.toBe(u);
    expect(copy.id).toBe("u1");
    expect(copy.time).toBe(3000);
    expect(copy.distance).toBe(20000);
    expect(copy.count).toBe(3);
  });

  it("subtracts to zero against itself", () => {
    const u = usage("u1", {
      time: 3000,
      duration: 3000,
      distance: 20000,
      climb: 400,
      descend: 280,
      energy: 2000,
      count: 3,
    });
    const zero = u.sub(u);
    expect(zero.id).toBe("u1");
    expect(zero.time).toBe(0);
    expect(zero.duration).toBe(0);
    expect(zero.distance).toBe(0);
    expect(zero.climb).toBe(0);
    expect(zero.descend).toBe(0);
    expect(zero.energy).toBe(0);
    expect(zero.count).toBe(0);
  });

  it("goes negative when the rhs is larger", () => {
    const u1 = usage("u1", {
      time: 1000,
      distance: 5000,
      climb: 100,
      count: 1,
    });
    const u2 = usage("u2", {
      time: 3000,
      distance: 20000,
      climb: 400,
      count: 3,
    });
    const diff = u1.sub(u2);
    expect(diff.time).toBe(-2000);
    expect(diff.distance).toBe(-15000);
    expect(diff.climb).toBe(-300);
    expect(diff.count).toBe(-2);
  });
});
