import { fireEvent, render } from "@testing-library/svelte";
import { tick } from "svelte";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import Activities from "./Activities.svelte";
import { activities } from "../lib/activity";
import { parts } from "../lib/part";
import { getTypes, setCategory } from "../lib/types.svelte";

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

/// Reset the reactive world to a clean slate and stub the browser APIs jsdom
/// lacks. Shared by every describe block: jsdom has no ResizeObserver and the
/// RangeSlider installs one in onMount, so a no-op stub is enough (the
/// reactivity under test is pure); the collections and the active category are
/// cleared so each test starts from an empty world.
function resetWorld() {
  class ResizeObserverStub {
    observe() {}
    unobserve() {}
    disconnect() {}
  }
  vi.stubGlobal("ResizeObserver", ResizeObserverStub);
  activities.setMap([]);
  parts.setMap([]);
  setCategory(undefined);
}

/// Mount the page and let Svelte's effect scheduler flush (five `tick()`
/// rounds settle the effect queue).
async function mountPage(params: { part: number }): Promise<HTMLElement> {
  const { container } = render(Activities, { params });
  for (let i = 0; i < 5; i++) await tick();
  return container;
}

/// Stub the type endpoints with a mocked fetch, drive the real `getTypes()`
/// (which populates the `types` map and sets the active category), and load
/// one gear (bike, id 5) — the shared prefix of every seeded world.
async function seedTypes() {
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
}

/// Seed the reactive world the way the app does after login: types plus one
/// gear (bike, id 5) and a mixed bag of activities — some on the bike,
/// several with no gear at all. The no-gear rides are the reporter's real
/// condition.
async function seedWorld() {
  await seedTypes();

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

/// Seed activities whose localized start dates differ in year, day, and
/// time of day, so each search disjunct (name, device, date) can be
/// exercised independently. In the runtime locale (en-US) and timezone
/// (Europe/Berlin) these render as:
///   Ride A -> 6/15/2023 10:00:00 AM, device "Garmin Edge 1040"
///   Ride B -> 5/1/2024  10:00:00 AM
///   Ride C -> 5/2/2024  8:00:00 PM
async function seedSearchWorld() {
  await seedTypes();

  activities.setMap([
    actData({
      id: "icu:200",
      name: "Ride A",
      start: "2023-06-15T08:00:00Z",
      device_name: "Garmin Edge 1040",
    }),
    actData({ id: "icu:201", name: "Ride B", start: "2024-05-01T08:00:00Z" }),
    actData({ id: "icu:202", name: "Ride C", start: "2024-05-02T18:00:00Z" }),
  ]);
}

/// Let the effect scheduler flush after a render or state change. The search
/// pipeline reads a debounced value, so a keystroke's filter only lands ~200ms
/// after typing pauses; wait past that, then settle the effect queue.
async function settle() {
  await new Promise((resolve) => setTimeout(resolve, 300));
  for (let i = 0; i < 5; i++) await tick();
}

/// Mount the page (see `mountPage`), type `query` into the search box, and
/// return the settled page text.
async function mountAndSearch(query: string): Promise<string> {
  await seedSearchWorld();
  const container = await mountPage({ part: 0 });

  const input = container.querySelector(
    'input[type="text"][placeholder="Search..."]',
  ) as HTMLInputElement;
  // Svelte 5's `bind:value` listens to `input`, not `change`.
  fireEvent.input(input, { target: { value: query } });
  await settle();
  return container.textContent ?? "";
}

/// Mount the page (see `mountPage`) and collect any reactivity-loop errors. A
/// reactivity loop throws `effect_update_depth_exceeded` mid-flush (dev mode);
/// in jsdom that surfaces on the window `error` event and/or via
/// `console.error`. We collect both so the assertion is robust to which path
/// Svelte takes.
async function mountAndCollectErrors(params: {
  part: number;
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
    const container = await mountPage(params);
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

describe("Activities page reactivity (effect_update_depth_exceeded)", () => {
  beforeEach(() => {
    resetWorld();
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
    const container = await mountPage({ part: 0 });
    await new Promise((resolve) => setTimeout(resolve, 25));

    // All four rides start within the initial range, so all are listed.
    expect(container.textContent).toContain("Ride A");
    expect(container.textContent).toContain("Ride B");

    // Nudge the start handle one day forward: the earliest ride (A) falls
    // out of the date range. This locks in that the slider drives the date
    // filter through the one-way `values` + `on:change` wiring.
    const handle = container.querySelector(".rangeHandle") as HTMLElement;
    handle.focus();
    handle.dispatchEvent(
      new KeyboardEvent("keydown", { key: "ArrowRight", bubbles: true }),
    );
    await settle();

    expect(container.textContent).not.toContain("Ride A");
    expect(container.textContent).toContain("Ride B");
  });
});

describe("Activities page search debounce", () => {
  beforeEach(() => {
    resetWorld();
    // The search debounce (setTimeout) and the Web-Animations stub in
    // setup.ts (which also runs on setTimeout) are driven by fake timers.
    vi.useFakeTimers({
      toFake: [
        "setTimeout",
        "clearTimeout",
        "setInterval",
        "clearInterval",
        "requestAnimationFrame",
        "cancelAnimationFrame",
        "performance",
      ],
    });
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  function searchInput(container: HTMLElement): HTMLInputElement {
    return container.querySelector("input[type='text']") as HTMLInputElement;
  }

  function typeSearch(container: HTMLElement, value: string) {
    const input = searchInput(container);
    input.value = value;
    input.dispatchEvent(new Event("input", { bubbles: true }));
  }

  it("does not re-filter on every keystroke; the filter runs ~200ms after typing pauses", async () => {
    await seedWorld();
    const container = await mountPage({ part: 0 });
    expect(container.textContent).toContain("Ride B");

    typeSearch(container, "Ride A");
    await tick();

    // Immediately after the keystroke the list is unchanged: the filter
    // pipeline has not re-run.
    expect(container.textContent).toContain("Ride B");

    // ~200ms after typing pauses, the filter runs.
    await vi.advanceTimersByTimeAsync(200);
    await tick();

    expect(container.textContent).toContain("Ride A");
    expect(container.textContent).not.toContain("Ride B");
  });

  it("clearing the search box updates the list immediately, with no debounce delay", async () => {
    await seedWorld();
    const container = await mountPage({ part: 0 });

    typeSearch(container, "Ride A");
    await vi.advanceTimersByTimeAsync(200);
    await tick();
    expect(container.textContent).not.toContain("Ride B");

    typeSearch(container, "");
    await tick(); // no timer advance

    expect(container.textContent).toContain("Ride B");
  });
});

describe("Activities page search", () => {
  beforeEach(() => {
    resetWorld();
  });

  it("a substring of the localized start date+time matches", async () => {
    const text = await mountAndSearch("5/1/2024");
    expect(text).toContain("Ride B");
    expect(text).not.toContain("Ride A");
    expect(text).not.toContain("Ride C");
  });

  it("a partial date fragment (year) matches", async () => {
    const text = await mountAndSearch("2023");
    expect(text).toContain("Ride A");
    expect(text).not.toContain("Ride B");
    expect(text).not.toContain("Ride C");
  });

  it("a partial date fragment (month/day) matches", async () => {
    // The runtime locale renders the month numerically ("5/1/2024"), so the
    // month fragment is the month/day prefix of the displayed date.
    const text = await mountAndSearch("5/1");
    expect(text).toContain("Ride B");
    expect(text).not.toContain("Ride A");
    expect(text).not.toContain("Ride C");
  });

  it("a time-of-day fragment matches", async () => {
    // 18:00Z renders as a PM time only in timezones where it lands in the
    // afternoon; pin the timezone the seed data was chosen for so the
    // fragment is stable everywhere.
    vi.stubEnv("TZ", "Europe/Berlin");
    const text = await mountAndSearch("PM");
    expect(text).toContain("Ride C");
    expect(text).not.toContain("Ride A");
    expect(text).not.toContain("Ride B");
  });

  it("name search matches a name substring, as before", async () => {
    const text = await mountAndSearch("Ride B");
    expect(text).toContain("Ride B");
    expect(text).not.toContain("Ride A");
    expect(text).not.toContain("Ride C");
  });

  it("device search matches a device name, as before", async () => {
    const text = await mountAndSearch("Garmin");
    expect(text).toContain("Ride A");
    expect(text).not.toContain("Ride B");
    expect(text).not.toContain("Ride C");
  });

  it("an empty search shows all activities", async () => {
    const text = await mountAndSearch("");
    expect(text).toContain("Ride A");
    expect(text).toContain("Ride B");
    expect(text).toContain("Ride C");
  });
});
