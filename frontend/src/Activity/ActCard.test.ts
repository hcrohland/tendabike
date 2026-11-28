import { render, screen } from "@testing-library/svelte";
import { describe, expect, it } from "vitest";
import { activity } from "../test/helpers";
import ActCard from "./ActCard.svelte";

describe("ActCard", () => {
  it("renders the card's secondary surface", () => {
    render(ActCard, { activity: activity() });
    const card = screen.getByText("Ride").closest(".surface-secondary");
    expect(card).toBeTruthy();
    // the card is the darker surface, not the light one
    expect(card?.classList.contains("surface-primary")).toBe(false);
  });

  it("renders its metric chips with the single uniform fill", () => {
    render(ActCard, { activity: activity() });
    const time = screen.getByText("1:00");
    // the uniform fill is one theme role class, not per-color
    const fill = time.closest(".fill");
    expect(fill).toBeTruthy();
    expect(time.classList.contains("text-white")).toBe(true);
  });
});
