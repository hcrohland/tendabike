import { fireEvent, render, screen, waitFor } from "@testing-library/svelte";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { notifications } from "../lib/notification.svelte";
import { Part, parts } from "../lib/part";
import { shops, setShop } from "../lib/shop";
import { setUser } from "../lib/user";
import { resp } from "../test/helpers";
import NotificationPanel from "./NotificationPanel.svelte";

let fetchMock: ReturnType<typeof vi.fn>;

beforeEach(() => {
  fetchMock = vi
    .fn()
    .mockImplementation(() => Promise.resolve(resp(null, 204, true)));
  vi.stubGlobal("fetch", fetchMock);
  vi.stubGlobal("location", { hash: "" });
  setUser(user);
  notifications.setMap([]);
  parts.setMap([]);
  shops.setMap([]);
  setShop(undefined);
});

const user = {
  id: 1,
  firstname: "Max",
  name: "Max Mustermann",
  avatar: undefined,
  is_admin: false,
  onboarding_status: "completed",
} as const;

function seedNotification(overrides: Partial<any> = {}) {
  notifications.setMap([
    {
      id: "n1",
      recipient: 1,
      lifecycle: "persistent",
      kind: "sub_request",
      subject: null,
      state: "new",
      created_at: "2026-01-01T00:00:00Z",
      ...overrides,
    },
  ]);
}

function seedPart(id: number, name: string, shop: number | null = null) {
  parts.setMap([
    new Part({
      id,
      owner: 1,
      what: 10,
      name,
      vendor: "",
      model: "",
      purchase: "2023-01-01T00:00:00Z",
      last_used: "2024-01-01T00:00:00Z",
      disposed_at: null,
      usage: "u1",
      shop,
    }),
  ]);
}

describe("NotificationPanel body precedence", () => {
  it("shows the notification's message in the body when one is present", () => {
    seedNotification({ message: "Please let me subscribe to your shop!" });
    render(NotificationPanel, { close: () => {} });
    expect(
      screen.getByText("Please let me subscribe to your shop!"),
    ).toBeTruthy();
  });

  it("shows the part's name in a part notification's body", () => {
    seedPart(5, "Wheel");
    seedNotification({
      kind: "part_registered",
      subject: { type: "part", id: 5 },
    });
    render(NotificationPanel, { close: () => {} });
    expect(screen.getByText("Wheel")).toBeTruthy();
  });

  it("falls back to the kind-based body when no message is present", () => {
    seedNotification({ kind: "sub_response" });
    render(NotificationPanel, { close: () => {} });
    expect(
      screen.getByText("A shop responded to your subscription request."),
    ).toBeTruthy();
  });

  it("prefers the message over the part name when both are present", () => {
    seedPart(5, "Wheel");
    seedNotification({
      kind: "part_registered",
      subject: { type: "part", id: 5 },
      message: "Your wheel was registered to a shop",
    });
    render(NotificationPanel, { close: () => {} });
    expect(
      screen.getByText("Your wheel was registered to a shop"),
    ).toBeTruthy();
    expect(screen.queryByText("Wheel")).toBeNull();
  });
});

describe("NotificationPanel navigation", () => {
  it("a subscription request navigates to the shops page on the shop-owner tab and marks it read", async () => {
    seedNotification({ kind: "sub_request" });
    render(NotificationPanel, { close: () => {} });
    fireEvent.click(screen.getByText("Shop subscription request"));
    expect(globalThis.location.hash).toBe("#/shops?tab=my-shops");
    await waitFor(() => {
      expect(fetchMock).toHaveBeenCalledWith(
        "/api/notification/n1/read",
        expect.anything(),
      );
    });
  });

  it("a subscription response navigates to the shops page on the my-subscriptions tab", () => {
    seedNotification({ kind: "sub_response" });
    render(NotificationPanel, { close: () => {} });
    fireEvent.click(screen.getByText("Shop subscription response"));
    expect(globalThis.location.hash).toBe("#/shops?tab=my-subscriptions");
  });

  it("a subscription cancelled entry navigates to the shops page on the shop-owner tab", () => {
    seedNotification({ kind: "sub_cancelled" });
    render(NotificationPanel, { close: () => {} });
    fireEvent.click(screen.getByText("Subscription cancelled"));
    expect(globalThis.location.hash).toBe("#/shops?tab=my-shops");
  });

  it("a part registered entry does not navigate (informational)", () => {
    seedPart(5, "Wheel", 10);
    seedNotification({
      kind: "part_registered",
      subject: { type: "part", id: 5 },
    });
    render(NotificationPanel, { close: () => {} });
    fireEvent.click(screen.getByText("Part registered"));
    expect(globalThis.location.hash).toBe("");
  });

  it("a part unregistered entry does not navigate (informational)", () => {
    seedPart(5, "Wheel", null);
    seedNotification({
      kind: "part_unregistered",
      subject: { type: "part", id: 5 },
    });
    render(NotificationPanel, { close: () => {} });
    fireEvent.click(screen.getByText("Part unregistered"));
    expect(globalThis.location.hash).toBe("");
  });

  it("the dismiss control dismisses without triggering navigation", async () => {
    seedNotification({ kind: "sub_request" });
    render(NotificationPanel, { close: () => {} });
    fireEvent.click(screen.getByRole("button", { name: "Dismiss" }));
    expect(globalThis.location.hash).toBe("");
    await waitFor(() => {
      expect(fetchMock).toHaveBeenCalledWith(
        "/api/notification/n1/read",
        expect.anything(),
      );
    });
  });
});
