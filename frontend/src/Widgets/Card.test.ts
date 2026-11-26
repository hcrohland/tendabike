import { render, screen } from "@testing-library/svelte";
import { describe, expect, it } from "vitest";
import CardHost from "./CardHost.svelte";

// The Card is the seam for the surface/tone flip: it renders one of the two
// neutral tones, flips a nested Card to the opposite tone, and provides its
// tone to its children (a Tone child renders the contrasting tone).
// We assert only the rendered tone class, never the mechanism.

describe("Card", () => {
  it("renders the darker tone by default", () => {
    render(CardHost, { mode: "default" });
    const card = screen.getByText("hello").closest(".bg-secondary-100");
    expect(card).toBeTruthy();
  });

  it("renders the light tone when light is set (its direction is preserved)", () => {
    render(CardHost, { mode: "light" });
    const card = screen.getByText("hello").closest(".bg-primary-100");
    expect(card).toBeTruthy();
  });

  it("renders the light surface when color is primary", () => {
    render(CardHost, { mode: "color-primary" });
    const card = screen.getByText("hello").closest(".bg-primary-100");
    expect(card).toBeTruthy();
  });

  it("renders the darker surface when color is secondary", () => {
    render(CardHost, { mode: "color-secondary" });
    const card = screen.getByText("hello").closest(".bg-secondary-100");
    expect(card).toBeTruthy();
    // the secondary card is the darker surface, not the light one
    expect(card?.classList.contains("bg-primary-100")).toBe(false);
  });

  it("flips a nested card to the opposite tone", () => {
    render(CardHost, { mode: "nested" });
    const nested = screen.getByText("nested").closest(".bg-primary-100");
    expect(nested).toBeTruthy();
    // the nested card is the light tone, not the darker one
    expect(nested?.classList.contains("bg-secondary-100")).toBe(false);
  });

  it("provides its tone to children, which contrast it", () => {
    render(CardHost, { mode: "tone" });
    // the Card renders the darker tone; the Tone child contrasts with the light tone
    const tone = screen.getByText("content").closest(".bg-primary-100");
    expect(tone).toBeTruthy();
  });

  it("renders the surface for its color when a Tone has a color", () => {
    render(CardHost, { mode: "tone-color-primary" });
    // the Tone's own div (not its enclosing Card) carries the color's surface
    const tone = screen.getByText("content");
    expect(tone.classList.contains("bg-primary-100")).toBe(true);
  });

  it("renders the darker surface for a secondary Tone color", () => {
    render(CardHost, { mode: "tone-color-secondary" });
    const tone = screen.getByText("content");
    expect(tone.classList.contains("bg-secondary-100")).toBe(true);
    // the Tone is the darker surface, not the light contrast of its Card
    expect(tone.classList.contains("bg-primary-100")).toBe(false);
  });
});
