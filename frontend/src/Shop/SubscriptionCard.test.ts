import { render, screen } from "@testing-library/svelte";
import { describe, expect, it } from "vitest";
import { ShopSubscription } from "../lib/subscription";
import SubscriptionCard from "./SubscriptionCard.svelte";

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

function subscriptionData(overrides: Partial<any> = {}): any {
  return {
    id: 100,
    shop_id: 10,
    user_id: 1,
    status: "pending",
    message: "Please let me subscribe!",
    response_message: "Sure, approved.",
    created_at: "2023-01-01T00:00:00Z",
    updated_at: "2023-01-02T00:00:00Z",
    shop: shopData(),
    owner: { id: 1, firstname: "Max", name: "Max Mustermann" },
    ...overrides,
  };
}

describe("SubscriptionCard", () => {
  it("shows the shop name, status, and date", () => {
    render(SubscriptionCard, {
      subscription: new ShopSubscription(subscriptionData()),
    });
    expect(screen.getByText("Velo Shop")).toBeTruthy();
    expect(screen.getByText("Pending")).toBeTruthy();
  });

  it("drops the request/response message", () => {
    render(SubscriptionCard, {
      subscription: new ShopSubscription(subscriptionData()),
    });
    expect(screen.queryByText("Please let me subscribe!")).toBeNull();
    expect(screen.queryByText("Sure, approved.")).toBeNull();
  });

  // The subscription card is a migrated call site: it renders an explicit
  // secondary surface and its status chip is a secondary fill. We assert only
  // the rendered surface/fill classes, never the mechanism.
  it("renders the card's secondary surface and its chip as a secondary fill", () => {
    const { container } = render(SubscriptionCard, {
      subscription: new ShopSubscription(subscriptionData()),
    });
    const card = container.querySelector(".rounded-lg");
    expect(card?.classList.contains("bg-secondary-100")).toBe(true);
    expect(card?.classList.contains("bg-primary-100")).toBe(false);
    const chip = screen.getByText("Pending");
    const chipRoot = chip.closest(".shrink-0");
    expect(chipRoot?.classList.contains("bg-secondary-700")).toBe(true);
    expect(chipRoot?.classList.contains("bg-secondary-100")).toBe(false);
    expect(chip.classList.contains("text-white")).toBe(true);
  });
});
