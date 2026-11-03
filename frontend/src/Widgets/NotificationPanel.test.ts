import { render, screen } from "@testing-library/svelte";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { notifications } from "../lib/notification.svelte";
import { Part, parts } from "../lib/part";
import { setUser } from "../lib/user";
import NotificationPanel from "./NotificationPanel.svelte";

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

function seedPart(id: number, name: string) {
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
      shop: null,
    }),
  ]);
}

describe("NotificationPanel body precedence", () => {
  beforeEach(() => {
    setUser(user);
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(undefined));
  });

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
