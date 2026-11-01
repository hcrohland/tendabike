import { fireEvent, render, screen } from "@testing-library/svelte";
import { flushSync } from "svelte";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { notifications, setPanelOpen } from "../lib/notification.svelte";
import { setUser } from "../lib/user";
import { resp } from "../test/helpers";
import NotificationBell from "./NotificationBell.svelte";

const user = {
  id: 1,
  firstname: "Max",
  name: "Max Mustermann",
  avatar: undefined,
  is_admin: false,
  onboarding_status: "completed",
} as const;

function seedNotification() {
  notifications.setMap([
    {
      id: "n1",
      recipient: 1,
      lifecycle: "persistent",
      kind: "part_registered",
      subject: null,
      state: "new",
      created_at: "2026-01-01T00:00:00Z",
    },
  ]);
}

describe("NotificationBell", () => {
  let fetchMock: ReturnType<typeof vi.fn>;

  beforeEach(() => {
    setUser(user);
    seedNotification();
    setPanelOpen(false);
    fetchMock = vi.fn().mockResolvedValue(resp(null, 204, true, ""));
    vi.stubGlobal("fetch", fetchMock);
  });

  it("closes the panel when read all is clicked", () => {
    render(NotificationBell);
    const bell = screen.getByRole("button", { name: "Notifications" });
    fireEvent.click(bell);
    flushSync();
    expect(screen.getByText("Mark all as read")).toBeTruthy();

    fireEvent.click(screen.getByText("Mark all as read"));
    flushSync();
    expect(screen.queryByText("Mark all as read")).toBeNull();
    expect(fetchMock).toHaveBeenCalledWith(
      "/api/notification/read-all",
      expect.anything(),
    );
  });

  it("closes the panel when the background is clicked", () => {
    render(NotificationBell);
    const bell = screen.getByRole("button", { name: "Notifications" });
    fireEvent.click(bell);
    flushSync();
    expect(screen.getByText("Mark all as read")).toBeTruthy();

    fireEvent.click(document.body);
    flushSync();
    expect(screen.queryByText("Mark all as read")).toBeNull();
  });

  it("stays open when a click lands inside the panel", () => {
    render(NotificationBell);
    const bell = screen.getByRole("button", { name: "Notifications" });
    fireEvent.click(bell);
    flushSync();
    expect(screen.getByText("Mark all as read")).toBeTruthy();

    // The notification row title is inside the panel.
    fireEvent.click(screen.getByText("Part registered"));
    flushSync();
    expect(screen.getByText("Mark all as read")).toBeTruthy();
  });

  it("toggles the panel open and closed by the bell", () => {
    render(NotificationBell);
    const bell = screen.getByRole("button", { name: "Notifications" });
    expect(screen.queryByText("Mark all as read")).toBeNull();

    fireEvent.click(bell);
    flushSync();
    expect(screen.getByText("Mark all as read")).toBeTruthy();

    fireEvent.click(bell);
    flushSync();
    expect(screen.queryByText("Mark all as read")).toBeNull();
  });
});
