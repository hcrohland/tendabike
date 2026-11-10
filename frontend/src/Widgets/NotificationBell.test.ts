import { fireEvent, render, screen } from "@testing-library/svelte";
import { flushSync } from "svelte";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { notifications } from "../lib/notification.svelte";
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
  provider: "strava",
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
    // The flowbite-svelte Popper toggles its trigger on a requestAnimationFrame
    // debounce that compares against performance.now(); both must be faked for
    // the toggle to complete deterministically.
    vi.useFakeTimers({
      toFake: [
        "setTimeout",
        "clearTimeout",
        "setInterval",
        "clearInterval",
        "Date",
        "requestAnimationFrame",
        "cancelAnimationFrame",
        "performance",
      ],
    });
    setUser(user);
    seedNotification();
    fetchMock = vi.fn().mockResolvedValue(resp(null, 204, true, ""));
    vi.stubGlobal("fetch", fetchMock);
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  /// Settle the panel. The Popper's trigger debounce (200ms, rAF) and the
  /// dropdown's fade transition (100ms, Web Animations API) each need timer
  /// advances, and Svelte schedules the animation on a microtask, so advance
  /// async (draining microtasks) and flush, a few times to cover every path.
  async function settle() {
    for (let i = 0; i < 3; i++) {
      await vi.advanceTimersByTimeAsync(300);
      flushSync();
    }
  }

  /// The Popper's trigger listens for mousedown (a real click produces one).
  async function clickBell(bell: Element) {
    fireEvent.mouseDown(bell);
    await settle();
  }

  it("closes the panel when read all is clicked", async () => {
    render(NotificationBell);
    const bell = screen.getByRole("button", { name: "Notifications" });
    await clickBell(bell);
    expect(screen.getByText("Mark all as read")).toBeTruthy();

    fireEvent.click(screen.getByText("Mark all as read"));
    await settle();
    expect(screen.queryByText("Mark all as read")).toBeNull();
    expect(fetchMock).toHaveBeenCalledWith(
      "/api/notification/read-all",
      expect.anything(),
    );
  });

  it("closes the panel when the background is clicked", async () => {
    render(NotificationBell);
    const bell = screen.getByRole("button", { name: "Notifications" });
    await clickBell(bell);
    expect(screen.getByText("Mark all as read")).toBeTruthy();

    fireEvent.click(document.body);
    await settle();
    expect(screen.queryByText("Mark all as read")).toBeNull();
  });

  it("stays open when a click lands inside the panel", async () => {
    render(NotificationBell);
    const bell = screen.getByRole("button", { name: "Notifications" });
    await clickBell(bell);
    expect(screen.getByText("Mark all as read")).toBeTruthy();

    // The notification row title is inside the panel.
    fireEvent.click(screen.getByText("Part registered"));
    await settle();
    expect(screen.getByText("Mark all as read")).toBeTruthy();
  });

  it("toggles the panel open and closed by the bell", async () => {
    render(NotificationBell);
    const bell = screen.getByRole("button", { name: "Notifications" });
    expect(screen.queryByText("Mark all as read")).toBeNull();

    await clickBell(bell);
    expect(screen.getByText("Mark all as read")).toBeTruthy();

    await clickBell(bell);
    expect(screen.queryByText("Mark all as read")).toBeNull();
  });
});
