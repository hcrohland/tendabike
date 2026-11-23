import { render, screen } from "@testing-library/svelte";
import { describe, expect, it } from "vitest";
import ShopMenu from "./ShopMenu.svelte";

/// The header shop menu's "Manage shops" entry must target the path-routed
/// /shops page. A leftover hash route (/#/shops) from the pre-SvelteKit SPA
/// router changes only the URL fragment, so the click navigates nowhere.
describe("ShopMenu", () => {
  it("links Manage shops to the path-routed /shops page", () => {
    render(ShopMenu);
    const link = screen.getByRole("link", { name: /Manage shops/i });
    expect(link.getAttribute("href")).toBe("/shops");
  });
});
