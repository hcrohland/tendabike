import { render, screen } from "@testing-library/svelte";
import { describe, expect, it } from "vitest";
import { Activity } from "../lib/activity";
import ActCard from "./ActCard.svelte";

// The activity card is a migrated call site: its surface is an explicit
// `secondary` color (not a tone read from context), and its metric chips are
// `secondary` fills. We assert only the rendered surface/fill classes, never
// the mechanism.
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

describe("ActCard", () => {
  it("renders the card's secondary surface", () => {
    render(ActCard, { activity: act() });
    const card = screen.getByText("Ride").closest(".bg-secondary-100");
    expect(card).toBeTruthy();
    // the card is the darker surface, not the light one
    expect(card?.classList.contains("bg-primary-100")).toBe(false);
  });

  it("renders its metric chips as secondary fills (dark step + white text)", () => {
    render(ActCard, { activity: act() });
    const time = screen.getByText("1:00");
    // a fill is the dark step with white text
    const fill = time.closest(".bg-secondary-700");
    expect(fill).toBeTruthy();
    expect(time.classList.contains("text-white")).toBe(true);
  });
});
