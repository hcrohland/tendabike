import { render, screen } from "@testing-library/svelte";
import { describe, expect, it } from "vitest";
import Example from "../../verify/Example.svelte";

// Smoke test for the component-verification harness (frontend/verify/).
// Mounts the harness example in jsdom and checks the surface/fill classes, so a
// broken import, $app/* stub, or entry is caught here when the stack moves.
// It is not a rendering check: the theme is verified in a real browser per
// docs/component-verify.md.
describe("verify harness", () => {
  it("mounts the migrated components with their surface/fill classes", () => {
    render(Example);

    // A card renders its color's surface; its chip is a fill of the same color.
    const card = screen.getByTestId("scenario-card");
    expect(card.querySelector(".bg-secondary-100")).toBeTruthy();
    expect(card.querySelector(".bg-secondary-700")).toBeTruthy();

    // A nested card flips: the outer is the secondary surface, the inner the
    // primary surface.
    const nested = screen.getByTestId("scenario-nested");
    expect(nested.querySelector(".bg-secondary-100")).toBeTruthy();
    expect(nested.querySelector(".bg-primary-100")).toBeTruthy();

    // The fill buttons render.
    expect(screen.getByTestId("fill-primary")).toBeTruthy();
    expect(screen.getByTestId("fill-secondary")).toBeTruthy();

    // The migrated plan block renders its secondary card with the nested
    // primary row inside.
    const planblock = screen.getByTestId("scenario-planblock");
    expect(planblock.querySelector(".bg-secondary-100")).toBeTruthy();
    expect(planblock.querySelector(".bg-primary-100")).toBeTruthy();

    // The migrated service history card renders its secondary surface.
    const servicehist = screen.getByTestId("scenario-servicehist");
    expect(servicehist.querySelector(".bg-secondary-100")).toBeTruthy();
  });
});
