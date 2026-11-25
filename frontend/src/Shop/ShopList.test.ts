import { render, screen } from "@testing-library/svelte";
import { describe, expect, it } from "vitest";
import { Shop } from "../lib/shop.svelte";
import ShopList from "./ShopList.svelte";

describe("ShopList", () => {
  it("renders the primary call-to-action in the muted info hue", () => {
    const owner = { id: 2, firstname: "Vera", name: "Velo", avatar: undefined };
    const shop = new Shop({
      id: 10,
      owner: 2,
      name: "Velo Shop",
      description: "",
      auto_approve: false,
      created_at: "2023-01-01T00:00:00Z",
    });
    render(ShopList, { shops: [shop], users: { 2: owner } });
    const cta = screen.getByRole("button", { name: "Request Subscription" });
    expect(cta.className).toContain("bg-blue-700");
  });
});
