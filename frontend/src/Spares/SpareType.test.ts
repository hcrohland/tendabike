import { fireEvent, render } from "@testing-library/svelte";
import { flushSync } from "svelte";
import { beforeEach, describe, expect, it } from "vitest";
import { Part, parts } from "../lib/part";
import { usages } from "../lib/usage";
import {
  expectFill,
  expectSurface,
  partData,
  standardUsage,
} from "../test/helpers";
import { Type } from "../lib/types.svelte";
import SpareType from "./SpareType.svelte";

describe("SpareType", () => {
  beforeEach(() => {
    parts.setMap([]);
    usages.setMap([]);
  });

  const type = new Type({
    id: 10,
    name: "Wheel",
    main: 1,
    hooks: [1],
    order: 1,
  });

  function seedPart(overrides: Partial<any> = {}) {
    parts.setMap([
      new Part(partData({ vendor: "Rider", model: "700c", ...overrides })),
    ]);
    usages.setMap([standardUsage({ count: 5 })]);
  }

  function renderSpareType() {
    return render(SpareType, {
      type,
      update: () => {},
      attachee: 0,
    });
  }

  it("renders the outer block on the secondary surface and the active part card on the primary surface, with the uniform chip fill", () => {
    seedPart();
    const { container } = renderSpareType();
    expectSurface(container.querySelector(".flex.flex-col"), "secondary");
    // the active part card is a nested block: the contrasting primary surface
    expectSurface(
      container.querySelector("div.flex.flex-col > div.rounded-lg"),
      "primary",
    );
    // the usage chip is the uniform fill, not the card's surface
    expectFill(container.querySelector(".relative.rounded-lg span.text-white"));
  });

  it("renders the disposed part card on the secondary surface with the uniform chip fill", () => {
    seedPart({ disposed_at: "2025-01-01T00:00:00Z" });
    const { container } = renderSpareType();
    // a disposed part is hidden until show-more is on
    const showMore = container.querySelector("button.rounded-full");
    expect(showMore).toBeTruthy();
    fireEvent.click(showMore!);
    flushSync();
    const partCard = container.querySelector(
      "div.flex.flex-col > div.rounded-lg",
    );
    // a disposed card is an exceptional layout: it composes the background
    // utility and a custom border directly, not the surface role class
    expect(partCard?.classList.contains("bg-secondary-100")).toBe(true);
    expect(partCard?.classList.contains("surface-secondary")).toBe(false);
    // the chip is the uniform fill regardless of the card's surface color
    expectFill(partCard!.querySelector(".relative.rounded-lg span.text-white"));
  });
});
