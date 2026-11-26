import { fireEvent, render } from "@testing-library/svelte";
import { flushSync } from "svelte";
import { beforeEach, describe, expect, it } from "vitest";
import { Part, parts } from "../lib/part";
import { usages } from "../lib/usage";
import { usage } from "../test/helpers";
import { Type } from "../lib/types.svelte";
import SpareType from "./SpareType.svelte";

// The spare-type selector is a migrated call site: its outer block renders an
// explicit secondary surface; each part card renders the primary surface while
// active and the secondary surface once disposed/mounted, and its usage chips
// are fills of the part card's color. We assert only the rendered
// surface/fill classes, never the mechanism.
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
      new Part({
        id: 7,
        owner: 1,
        what: 10,
        name: "Wheel",
        vendor: "Rider",
        model: "700c",
        purchase: "2023-01-01T00:00:00Z",
        last_used: "2024-01-01T00:00:00Z",
        usage: "u1",
        ...overrides,
      }),
    ]);
    usages.setMap([
      usage("u1", {
        count: 5,
        distance: 1000,
        time: 3600,
        duration: 3600,
        energy: 100,
      }),
    ]);
  }

  function renderSpareType() {
    return render(SpareType, {
      type,
      update: () => {},
      attachee: 0,
    });
  }

  it("renders the outer block on the secondary surface and the active part card on the primary surface with a primary chip fill", () => {
    seedPart();
    const { container } = renderSpareType();
    const outer = container.querySelector(".flex.flex-col");
    expect(outer?.classList.contains("bg-secondary-100")).toBe(true);
    expect(outer?.classList.contains("bg-primary-100")).toBe(false);
    // the active part card is a nested block: the contrasting primary surface
    const partCard = container.querySelector(
      "div.flex.flex-col > div.rounded-lg",
    );
    expect(partCard?.classList.contains("bg-primary-100")).toBe(true);
    expect(partCard?.classList.contains("bg-secondary-100")).toBe(false);
    // the usage chips are primary fills: dark-step background + white text
    const chip = container.querySelector(".relative.rounded-lg")!;
    expect(chip.classList.contains("bg-primary-700")).toBe(true);
    expect(chip.classList.contains("bg-primary-100")).toBe(false);
  });

  it("renders the disposed part card on the secondary surface with a secondary chip fill", () => {
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
    expect(partCard).toBeTruthy();
    expect(partCard?.classList.contains("bg-secondary-100")).toBe(true);
    expect(partCard?.classList.contains("bg-primary-100")).toBe(false);
    const chip = partCard!.querySelector(".relative.rounded-lg")!;
    expect(chip.classList.contains("bg-secondary-700")).toBe(true);
    expect(chip.classList.contains("bg-secondary-100")).toBe(false);
  });
});
