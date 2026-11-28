import { render, screen } from "@testing-library/svelte";
import { describe, expect, it } from "vitest";
import { ShopSubscription } from "../lib/subscription";
import { expectFill, expectSurface } from "../test/helpers";
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

  it("renders the card's secondary surface and its chip with the uniform fill", () => {
    const { container } = render(SubscriptionCard, {
      subscription: new ShopSubscription(subscriptionData()),
    });
    expectSurface(container.querySelector(".rounded-lg"), "secondary");
    expectFill(screen.getByText("Pending"));
  });
});
