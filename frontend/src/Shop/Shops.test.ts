import { render, screen } from "@testing-library/svelte";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { shops, Shop } from "../lib/shop.svelte";
import { setUser } from "../lib/user";
import { resp } from "../test/helpers";
import Shops from "./Shops.svelte";

const user = {
  id: 1,
  firstname: "Max",
  name: "Max Mustermann",
  avatar: undefined,
  is_admin: false,
  onboarding_status: "completed",
  provider: "strava",
} as const;

beforeEach(() => {
  vi.stubGlobal(
    "fetch",
    vi.fn().mockImplementation(() => Promise.resolve(resp([], 200, true))),
  );
  setUser(user);
  shops.setMap([]);
});

/// Seed a shop owned by the current user so the shop-owner (my-shops) tab is
/// open and selectable.
function seedOwnedShop(id: number) {
  shops.setMap([
    new Shop({
      id,
      owner: 1,
      name: "Shop " + id,
      description: "",
      auto_approve: false,
      created_at: "2023-01-01T00:00:00Z",
    }),
  ]);
}

describe("Shops tab selection", () => {
  it("defaults to the subscriptions tab when no tab selector is supplied", async () => {
    render(Shops, { query: {} });
    await vi.waitFor(() => {
      expect(screen.getByRole("tab", { selected: true }).textContent).toBe(
        "My Subscriptions",
      );
    });
  });

  it("selects the my-shops tab when the route's tab selector is my-shops", async () => {
    seedOwnedShop(10);
    render(Shops, { query: { tab: "my-shops" } });
    await vi.waitFor(() => {
      expect(screen.getByRole("tab", { selected: true }).textContent).toBe(
        "My Shops",
      );
    });
  });

  it("selects the my-subscriptions tab when the route's tab selector is my-subscriptions", async () => {
    seedOwnedShop(10);
    render(Shops, { query: { tab: "my-subscriptions" } });
    await vi.waitFor(() => {
      expect(screen.getByRole("tab", { selected: true }).textContent).toBe(
        "My Subscriptions",
      );
    });
  });

  it("falls back to the subscriptions tab when the tab selector is not a known tab", async () => {
    seedOwnedShop(10);
    render(Shops, { query: { tab: "bogus" } });
    await vi.waitFor(() => {
      expect(screen.getByRole("tab", { selected: true }).textContent).toBe(
        "My Subscriptions",
      );
    });
  });
});
