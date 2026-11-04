import { render, screen } from "@testing-library/svelte";
import { describe, expect, it } from "vitest";
import { ShopSubscription } from "../lib/subscription";
import SubscriptionRow from "./SubscriptionRow.svelte";

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

describe("SubscriptionRow (de-messaged)", () => {
  it("drops the request/response message columns from the header", () => {
    render(SubscriptionRow);
    expect(screen.getByText("Shop")).toBeTruthy();
    expect(screen.getByText("Status")).toBeTruthy();
    expect(screen.getByText("Date")).toBeTruthy();
    expect(screen.getByText("Actions")).toBeTruthy();
    expect(screen.queryByText("Request Message")).toBeNull();
    expect(screen.queryByText("Response")).toBeNull();
  });

  it("drops the request/response message from a row", () => {
    render(SubscriptionRow, {
      subscription: new ShopSubscription(subscriptionData()),
    });
    expect(screen.queryByText("Please let me subscribe!")).toBeNull();
    expect(screen.queryByText("Sure, approved.")).toBeNull();
    // The remaining columns — shop, status, date — are still present.
    expect(screen.getByText("Velo Shop")).toBeTruthy();
    expect(screen.getByText("Pending")).toBeTruthy();
  });
});
