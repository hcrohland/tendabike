import { render, screen } from "@testing-library/svelte";
import { describe, expect, it } from "vitest";
import CardHost from "./CardHost.svelte";

// The Card renders the surface (background + border) of its `color`:
// `primary` is the light surface, `secondary` the darker one, and a
// top-level card defaults to `secondary`. A nested card is authored with
// the opposite color by its caller — there is no context and no automatic
// flip. We assert only the rendered surface class, never the mechanism.

describe("Card", () => {
  it("renders the darker surface by default", () => {
    render(CardHost, { mode: "default" });
    const card = screen.getByText("hello").closest(".bg-secondary-100");
    expect(card).toBeTruthy();
    // the default card is the darker surface, not the light one
    expect(card?.classList.contains("bg-primary-100")).toBe(false);
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

  it("renders a nested card in the opposite color (authored by the caller)", () => {
    render(CardHost, { mode: "nested" });
    const nested = screen.getByText("nested").closest(".bg-primary-100");
    expect(nested).toBeTruthy();
    // the nested card is the light surface, not the darker one
    expect(nested?.classList.contains("bg-secondary-100")).toBe(false);
  });

  it("renders the light surface for a Tone with a primary color", () => {
    render(CardHost, { mode: "tone-color-primary" });
    // the Tone's own div (not its enclosing Card) carries the color's surface
    const tone = screen.getByText("content");
    expect(tone.classList.contains("bg-primary-100")).toBe(true);
  });

  it("renders the darker surface for a Tone with a secondary color", () => {
    render(CardHost, { mode: "tone-color-secondary" });
    const tone = screen.getByText("content");
    expect(tone.classList.contains("bg-secondary-100")).toBe(true);
    // the Tone is the darker surface, not the light one
    expect(tone.classList.contains("bg-primary-100")).toBe(false);
  });
});
