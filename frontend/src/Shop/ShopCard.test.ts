import { render, screen } from "@testing-library/svelte";
import { beforeEach, describe, it, vi } from "vitest";
import { Shop } from "../lib/shop.svelte";
import { expectFill, expectSurface, resp } from "../test/helpers";
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

describe("ShopCard", () => {
  beforeEach(() => {
    vi.stubGlobal(
      "fetch",
      vi.fn().mockImplementation(() => Promise.resolve(resp([], 200, true))),
    );
  });

  it("renders the card's secondary surface and its chips with the uniform fill", () => {
    const { container } = render(ShopCard, {
      shop: new Shop(shopData()),
      users: { 1: owner },
      isOwner: true,
    });
    expectSurface(container.querySelector(".rounded-lg"), "secondary");
    expectFill(screen.getByText("Owner"));
  });
});
