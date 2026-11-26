import { render, screen } from "@testing-library/svelte";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { Shop } from "../lib/shop.svelte";
import { resp } from "../test/helpers";
import ShopCard from "./ShopCard.svelte";

const owner = {
  id: 1,
  firstname: "Max",
  name: "Mustermann",
  avatar: undefined,
};

function shopData(overrides: Partial<any> = {}): any {
  return {
    id: 10,
    owner: 1,
    name: "Velo Shop",
    description: "Local shop",
    auto_approve: true,
    created_at: "2023-01-01T00:00:00Z",
    ...overrides,
  };
}

// The shop card is a migrated call site: it renders an explicit secondary
// surface and its chips are secondary fills. We assert only the rendered
// surface/fill classes, never the mechanism.
describe("ShopCard", () => {
  beforeEach(() => {
    vi.stubGlobal(
      "fetch",
      vi.fn().mockImplementation(() => Promise.resolve(resp([], 200, true))),
    );
  });

  it("renders the card's secondary surface and its chips as secondary fills", () => {
    const { container } = render(ShopCard, {
      shop: new Shop(shopData()),
      users: { 1: owner },
      isOwner: true,
    });
    const card = container.querySelector(".rounded-lg");
    expect(card?.classList.contains("bg-secondary-100")).toBe(true);
    expect(card?.classList.contains("bg-primary-100")).toBe(false);
    // the owner chip is a fill: dark-step background + white text
    const chip = screen.getByText("Owner");
    const chipRoot = chip.closest(".shrink-0");
    expect(chipRoot?.classList.contains("bg-secondary-700")).toBe(true);
    expect(chipRoot?.classList.contains("bg-secondary-100")).toBe(false);
    expect(chip.classList.contains("text-white")).toBe(true);
  });
});
