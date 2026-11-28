import { render, screen } from "@testing-library/svelte";
import { describe, expect, it } from "vitest";
import { activity, expectFill } from "../test/helpers";
import ActCard from "./ActCard.svelte";

describe("ActCard", () => {
  it("renders the card's primary surface", () => {
    render(ActCard, { activity: activity() });
    const card = screen.getByText("Ride").closest(".surface-primary");
    expect(card).toBeTruthy();
    // the card is the light surface, not the darker one
    expect(card?.classList.contains("surface-secondary")).toBe(false);
  });

  it("renders its metric chips with the single uniform fill", () => {
    render(ActCard, { activity: activity() });
    expectFill(screen.getByText("1:00"));
  });
});
