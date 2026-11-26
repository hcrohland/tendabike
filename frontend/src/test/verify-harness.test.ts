import { render, screen } from "@testing-library/svelte";
import { describe, expect, it } from "vitest";
import Example from "../../verify/Example.svelte";

// Smoke test for the component-verification harness (frontend/verify/).
// Mounts the harness example in jsdom and checks the tone classes, so a
// broken import, $app/* stub, or entry is caught here when the stack moves.
// It is not a rendering check: the theme is verified in a real browser per
// docs/component-verify.md.
describe("verify harness", () => {
  it("mounts the example with the tone flip intact", () => {
    render(Example);

    // A top-level Card defaults to the darker tone; its content (Tone)
    // contrasts in the light tone.
    const def = screen.getByTestId("scenario-default");
    const card = def.querySelector("div.border");
    expect(card?.className).toContain("bg-secondary-100");
    const tone = def.querySelector("div.rounded-lg:not(.border)");
    expect(tone?.className).toContain("bg-primary-100");

    // A nested Card flips to the opposite tone.
    const nested = screen.getByTestId("scenario-nested");
    const cards = nested.querySelectorAll("div.border");
    expect(cards).toHaveLength(2);
    expect(cards[0]?.className).toContain("bg-secondary-100");
    expect(cards[1]?.className).toContain("bg-primary-100");
  });
});
