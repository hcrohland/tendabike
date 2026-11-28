import { render, screen } from "@testing-library/svelte";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { activity, expectFill } from "../test/helpers";
import TotalsCard from "./TotalsCard.svelte";

class ResizeObserverStub {
  observe() {}
  unobserve() {}
  disconnect() {}
}

const props = {
  title: "Activity Totals",
  activities: [activity()],
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
      .closest(".surface-secondary");
    expect(card).toBeTruthy();
    expect(card?.classList.contains("surface-primary")).toBe(false);
  });

  it("renders its metric chips with the single uniform fill", () => {
    render(TotalsCard, props);
    expectFill(screen.getByText("1:00"));
  });

  it("renders its search input on the contrasting (primary) surface", () => {
    render(TotalsCard, props);
    const input = screen.getByPlaceholderText("Search...");
    // a nested control flips to the opposite of the card's secondary surface
    expect(input.classList.contains("surface-primary")).toBe(true);
  });
});
