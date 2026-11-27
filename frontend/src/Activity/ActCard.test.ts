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

  it("renders its metric chips with the single uniform fill", () => {
    render(ActCard, { activity: activity() });
    const time = screen.getByText("1:00");
    // the uniform fill is one value, not per-color: primary-500 in light mode
    const fill = time.closest(".bg-primary-500");
    expect(fill).toBeTruthy();
    expect(time.classList.contains("text-white")).toBe(true);
  });
});
