import { render, screen } from "@testing-library/svelte";
import { flushSync } from "svelte";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { parts, Part } from "../lib/part";
import { services, Service } from "../lib/service";
import { usages } from "../lib/usage";
import { usage } from "../test/helpers";
import ServiceRow from "./ServiceRow.svelte";

describe("ServiceRow", () => {
  beforeEach(() => {
    parts.setMap([]);
    usages.setMap([]);
    services.setMap([]);
    // The window ends "now" without a successor; pin now so the days
    // text is deterministic.
    vi.useFakeTimers();
    vi.setSystemTime(new Date("2024-06-01T00:00:00Z"));
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  function seed() {
    const part = new Part({
      id: 7,
      owner: 1,
      what: 10,
      name: "Wheel",
      purchase: "2023-01-01T00:00:00Z",
      last_used: "2024-01-01T00:00:00Z",
      usage: "u_now",
    });
    const service = new Service({
      id: "S1",
      part_id: 7,
      time: "2024-01-01T00:00:00Z",
      redone: "2023-01-01T00:00:00Z",
      name: "Annual Service",
      notes: "",
      usage: "u_prev",
      successor: null,
      plans: [],
    });
    parts.setMap([part]);
    services.setMap([service]);
    usages.setMap([
      usage("u_prev", {
        count: 1,
        distance: 1000,
        time: 3600,
        duration: 3600,
        energy: 100,
      }),
      usage("u_now", {
        count: 3,
        distance: 31000,
        time: 10800,
        duration: 10800,
        energy: 300,
      }),
    ]);
    return { part, service };
  }

  it("re-renders the usage window when the usages collection changes", () => {
    const { part, service } = seed();
    const { unmount } = render(ServiceRow, { part, service });

    // Window from the service (2024-01-01) to now (2024-06-01): 152 days,
    // usage u_now - u_prev.
    expect(screen.getByText("152 days")).toBeTruthy();
    expect(screen.getByText("2")).toBeTruthy(); // rides
    expect(screen.getByText("2:00")).toBeTruthy(); // hours
    expect(screen.getByText("30")).toBeTruthy(); // km
    expect(screen.getByText("200")).toBeTruthy(); // kJ

    // The row reads usages through Service.period inside a $derived; the
    // mutation from this plain context must reach the DOM.
    usages.updateMap([
      usage("u_now", {
        count: 5,
        distance: 51000,
        time: 18000,
        duration: 18000,
        energy: 500,
      }),
    ]);
    flushSync();

    expect(screen.getByText("152 days")).toBeTruthy();
    expect(screen.getByText("4")).toBeTruthy();
    expect(screen.getByText("4:00")).toBeTruthy();
    expect(screen.getByText("50")).toBeTruthy();
    expect(screen.getByText("400")).toBeTruthy();
    expect(screen.queryByText("2")).toBeNull();
    expect(screen.queryByText("2:00")).toBeNull();
    expect(screen.queryByText("30")).toBeNull();
    expect(screen.queryByText("200")).toBeNull();
    unmount();
  });
});
