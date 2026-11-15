import { render } from "@testing-library/svelte";
import { tick } from "svelte";
import { beforeEach, describe, expect, it, vi } from "vitest";
import Activities from "./Activities.svelte";
import { activities } from "../lib/activity";
import { parts } from "../lib/part";
import { getTypes, setCategory } from "../lib/types";

function json200(body: unknown) {
  return {
    ok: true,
    status: 200,
    statusText: "OK",
    json: async () => body,
  } as unknown as Response;
}

/// Raw wire data, `any`-typed so it passes the `setMap` type check the way the
/// app's stream frames do; the entity constructors read the fields at runtime.
function partData(overrides: Record<string, unknown> = {}): any {
  return {
    id: 5,
    owner: 1,
    what: 1,
    name: "Bike A",
    vendor: "",
    model: "",
    purchase: "2023-01-01T00:00:00Z",
    last_used: "2024-01-01T00:00:00Z",
    disposed_at: null,
    usage: "u1",
    notes: "",
    shop: null,
    ...overrides,
  };
}

function actData(overrides: Record<string, unknown> = {}): any {
  return {
    id: "icu:100",
    user_id: 1,
    what: 301,
    name: "Ride",
    start: "2024-05-01T08:00:00Z",
    gear: 5,
    distance: 20000,
    time: 3600,
    duration: 3600,
    ...overrides,
  };
}

/// Seed the reactive world the way the app does after login: drive the real
/// `getTypes()` (which populates the `types` map and sets the active category)
/// with a mocked fetch, then load one gear (bike, id 5) and a mixed bag of
/// activities — some on the bike, several with no gear at all. The no-gear
/// rides are the reporter's real condition.
async function seedWorld() {
  vi.stubGlobal(
    "fetch",
    vi.fn((url: string) => {
      if (url === "/api/types/part")
        return Promise.resolve(
          json200([{ id: 1, name: "Bike", main: 1, hooks: [], order: 1 }]),
        );
      if (url === "/api/types/activity")
        return Promise.resolve(
          json200([{ id: 301, name: "Ride", gear_type: 1 }]),
        );
      return Promise.resolve(json200([]));
    }),
  );
  await getTypes();

  parts.setMap([partData()]);

  // A mix: two rides on the bike, two with no gear assigned (the reporter's
  // real condition).
  activities.setMap([
    actData({ id: "icu:100", name: "Ride A", start: "2024-05-01T08:00:00Z" }),
    actData({ id: "icu:101", name: "Ride B", start: "2024-05-03T08:00:00Z" }),
    actData({
      id: "icu:102",
      name: "Ride C",
      start: "2024-05-05T08:00:00Z",
      gear: null,
    }),
    actData({
      id: "icu:103",
      name: "Ride D",
      start: "2024-05-07T08:00:00Z",
      gear: null,
    }),
  ]);
}

/// Mount the page and let Svelte's effect scheduler flush. A reactivity loop
/// throws `effect_update_depth_exceeded` mid-flush (dev mode); in jsdom that
/// surfaces on the window `error` event and/or via `console.error`. We collect
/// both so the assertion is robust to which path Svelte takes.
async function mountAndCollectErrors(params: {
  part: number;
  start?: number;
}): Promise<{ text: string; errors: string[] }> {
  const errors: string[] = [];
  const onWindowError = (e: ErrorEvent) => errors.push(String(e?.message ?? e));
  const consoleErr = vi
    .spyOn(console, "error")
    .mockImplementation((...args: unknown[]) =>
      errors.push(args.map((a) => String(a)).join(" ")),
    );
  window.addEventListener("error", onWindowError);
  try {
    const { container } = render(Activities, { params });
    for (let i = 0; i < 5; i++) await tick();
    await new Promise((resolve) => setTimeout(resolve, 25));
    return { text: container.textContent ?? "", errors };
  } finally {
    window.removeEventListener("error", onWindowError);
    consoleErr.mockRestore();
  }
}

function depthErrors(errors: string[]) {
  return errors.filter((e) => e.includes("effect_update_depth_exceeded"));
}

describe("Activities page (#893: icu activities with a bike → effect_update_depth_exceeded)", () => {
  beforeEach(() => {
    // jsdom has no ResizeObserver; the RangeSlider installs one in onMount.
    // A no-op stub is enough — the reactivity under test is pure.
    class ResizeObserverStub {
      observe() {}
      unobserve() {}
      disconnect() {}
    }
    vi.stubGlobal("ResizeObserver", ResizeObserverStub);
    activities.setMap([]);
    parts.setMap([]);
    setCategory(undefined);
  });

  it("bike view renders without exceeding the effect update depth", async () => {
    await seedWorld();
    const { text, errors } = await mountAndCollectErrors({ part: 5 });
    expect(depthErrors(errors), `depth errors: ${errors.join(" | ")}`).toEqual(
      [],
    );
    expect(text).toContain("Ride A");
  });

  it("all-activities view renders without exceeding the effect update depth", async () => {
    await seedWorld();
    const { text, errors } = await mountAndCollectErrors({ part: 0 });
    expect(depthErrors(errors), `depth errors: ${errors.join(" | ")}`).toEqual(
      [],
    );
    expect(text).toContain("Ride A");
  });

  it("moving the date slider filters the list through the one-way on:change wiring", async () => {
    await seedWorld();
    const { container } = render(Activities, { params: { part: 0 } });
    for (let i = 0; i < 5; i++) await tick();
    await new Promise((resolve) => setTimeout(resolve, 25));

    // All four rides start within the initial range, so all are listed.
    expect(container.textContent).toContain("Ride A");
    expect(container.textContent).toContain("Ride B");

    // Nudge the start handle one day forward: the earliest ride (A) falls
    // out of the date range. This locks in that the slider still drives the
    // date filter after replacing the two-way `bind:values` with the one-way
    // `values` + `on:change` wiring — the interaction must keep working.
    const handle = container.querySelector(".rangeHandle") as HTMLElement;
    handle.focus();
    handle.dispatchEvent(
      new KeyboardEvent("keydown", { key: "ArrowRight", bubbles: true }),
    );
    for (let i = 0; i < 5; i++) await tick();
    await new Promise((resolve) => setTimeout(resolve, 25));

    expect(container.textContent).not.toContain("Ride A");
    expect(container.textContent).toContain("Ride B");
  });
});
