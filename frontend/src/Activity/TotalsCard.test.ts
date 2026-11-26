import { render, screen } from "@testing-library/svelte";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { Activity } from "../lib/activity";
import TotalsCard from "./TotalsCard.svelte";

// The totals card is a migrated call site: its surface is an explicit
// `secondary` color, its nested controls (search input) take the contrasting
// `primary` surface, and its metric chips are `secondary` fills. We assert
// only the rendered surface/fill classes, never the mechanism.
class ResizeObserverStub {
  observe() {}
  unobserve() {}
  disconnect() {}
}

function act() {
  return new Activity({
    id: "icu:100",
    user_id: 1,
    what: 301,
    name: "Ride",
    start: "2024-05-01T08:00:00Z",
    distance: 20000,
    time: 3600,
    duration: 3600,
    climb: 100,
    descend: 80,
    energy: 50,
  });
}

const props = {
  title: "Activity Totals",
  activities: [act()],
  filterOptions: undefined,
  dateValues: [0, 1],
  min: 0,
  max: 1,
  gearFilter: undefined,
  searchText: "",
  sortBy: "start",
  sortDir: -1,
};

describe("TotalsCard", () => {
  beforeEach(() => {
    // The RangeSlider installs a ResizeObserver in onMount; jsdom lacks one.
    vi.stubGlobal("ResizeObserver", ResizeObserverStub);
  });

  it("renders the card's secondary surface", () => {
    render(TotalsCard, props);
    const card = screen
      .getByText("Activity Totals")
      .closest(".bg-secondary-100");
    expect(card).toBeTruthy();
    expect(card?.classList.contains("bg-primary-100")).toBe(false);
  });

  it("renders its metric chips as secondary fills (dark step + white text)", () => {
    render(TotalsCard, props);
    const time = screen.getByText("1:00");
    // a fill is the dark step with white text
    expect(time.closest(".bg-secondary-700")).toBeTruthy();
    expect(time.classList.contains("text-white")).toBe(true);
  });

  it("renders its search input on the contrasting (primary) surface", () => {
    render(TotalsCard, props);
    const input = screen.getByPlaceholderText("Search...");
    // a nested control flips to the opposite of the card's secondary surface
    expect(input.classList.contains("bg-primary-100")).toBe(true);
  });
});
