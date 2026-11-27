import { render, screen } from "@testing-library/svelte";
import { describe, expect, it } from "vitest";
import { activity } from "../test/helpers";
import ActCard from "./ActCard.svelte";

describe("ActCard", () => {
  it("renders the card's secondary surface", () => {
    render(ActCard, { activity: activity() });
    const card = screen.getByText("Ride").closest(".bg-secondary-100");
    expect(card).toBeTruthy();
    // the card is the darker surface, not the light one
    expect(card?.classList.contains("bg-primary-100")).toBe(false);
  });

  it("renders its metric chips as secondary fills (dark step + white text)", () => {
    render(ActCard, { activity: activity() });
    const time = screen.getByText("1:00");
    // a fill is the dark step with white text
    const fill = time.closest(".bg-secondary-700");
    expect(fill).toBeTruthy();
    expect(time.classList.contains("text-white")).toBe(true);
  });
});
