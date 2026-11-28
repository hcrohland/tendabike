import { render, screen } from "@testing-library/svelte";
import { describe, expect, it, vi } from "vitest";
import Example from "../../verify/Example.svelte";

// Smoke test for the component-verification harness (frontend/verify/).
// Mounts the harness example in jsdom and checks the surface/fill role
// classes, so a broken import, $app/* stub, or entry is caught here when the
// stack moves. It is not a rendering check: the theme is verified in a real
// browser per docs/component-verify.md.
describe("verify harness", () => {
  it("mounts the harness example with its surface/fill role classes", async () => {
    render(Example);

    // A card renders its color's surface; its chip is the single uniform fill.
    const card = screen.getByTestId("scenario-card");
    expect(card.querySelector(".surface-secondary")).toBeTruthy();
    expect(card.querySelector(".fill")).toBeTruthy();

    // A nested card flips: the outer is the secondary surface, the inner the
    // primary surface.
    const nested = screen.getByTestId("scenario-nested");
    expect(nested.querySelector(".surface-secondary")).toBeTruthy();
    expect(nested.querySelector(".surface-primary")).toBeTruthy();

    // The fill buttons render.
    expect(screen.getByTestId("fill-primary")).toBeTruthy();
    expect(screen.getByTestId("fill-secondary")).toBeTruthy();

    // The plan block renders its secondary card with the nested
    // primary row inside.
    const planblock = screen.getByTestId("scenario-planblock");
    expect(planblock.querySelector(".surface-secondary")).toBeTruthy();
    expect(planblock.querySelector(".surface-primary")).toBeTruthy();

    // The service history card renders its secondary surface.
    const servicehist = screen.getByTestId("scenario-servicehist");
    expect(servicehist.querySelector(".surface-secondary")).toBeTruthy();

    // The shop card renders its secondary surface with the uniform
    // chip fill.
    const shopcard = screen.getByTestId("scenario-shopcard");
    expect(shopcard.querySelector(".surface-secondary")).toBeTruthy();
    expect(shopcard.querySelector(".fill")).toBeTruthy();

    // The spare-type selector renders its secondary outer surface
    // with the active part card on the contrasting primary surface and the
    // uniform chip fill.
    const sparetype = screen.getByTestId("scenario-sparetype");
    expect(sparetype.querySelector(".surface-secondary")).toBeTruthy();
    expect(sparetype.querySelector(".surface-primary")).toBeTruthy();
    expect(sparetype.querySelector(".fill")).toBeTruthy();

    // The admin card renders its secondary surface with the
    // uniform chip fill.
    const admincard = screen.getByTestId("scenario-admincard");
    expect(admincard.querySelector(".surface-secondary")).toBeTruthy();
    expect(admincard.querySelector(".fill")).toBeTruthy();

    // The offline banner renders its secondary surface once the (stubbed)
    // providers fetch settles with an empty enabled set.
    await vi.waitFor(() => {
      expect(
        screen
          .getByTestId("scenario-offline")
          .querySelector(".surface-secondary"),
      ).toBeTruthy();
    });
  });
});
