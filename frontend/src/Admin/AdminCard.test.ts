import { fireEvent, render, screen } from "@testing-library/svelte";
import { flushSync } from "svelte";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { AdminRow, User } from "../lib/user.svelte";
import { resp } from "../test/helpers";
import AdminCard from "./AdminCard.svelte";

const user: User = {
  id: 3,
  firstname: "Max",
  name: "Mustermann",
  avatar: undefined,
  is_admin: false,
  onboarding_status: "completed",
  provider: "strava",
};

function renderCard(overrides: Record<string, any> = {}) {
  const createSync = vi.fn();
  const deleteUser = vi.fn();
  const refresh = vi.fn();
  const row: AdminRow = {
    user,
    parts: 2,
    activities: 7,
    events: 11,
    disabled: false,
    provider: "strava",
    ...overrides,
  };
  const utils = render(AdminCard, {
    props: { row, refresh, createSync, deleteUser },
  });
  return { utils, createSync, deleteUser, refresh };
}

describe("AdminCard", () => {
  let fetchMock: ReturnType<typeof vi.fn>;

  beforeEach(() => {
    fetchMock = vi.fn().mockResolvedValue(resp(null, 204, true, ""));
    vi.stubGlobal("fetch", fetchMock);
  });

  it("shows the user's name, role, and the parts/activities/events counts", () => {
    renderCard();
    expect(screen.getByText("Max Mustermann")).toBeTruthy();
    expect(screen.getByText("User")).toBeTruthy();
    expect(screen.getByText("2")).toBeTruthy();
    expect(screen.getByText("7")).toBeTruthy();
    expect(screen.getByText("11")).toBeTruthy();
  });

  it("shows the disabled role for a disabled user", () => {
    renderCard({ disabled: true });
    expect(screen.getByText("Disabled")).toBeTruthy();
  });

  // The admin card is a migrated call site: it renders an explicit secondary
  // surface and its count chips are secondary fills. We assert only the
  // rendered surface/fill classes, never the mechanism.
  it("renders the card's secondary surface and its chips as secondary fills", () => {
    const { utils } = renderCard();
    const card = utils.container.querySelector(".rounded-lg");
    expect(card?.classList.contains("bg-secondary-100")).toBe(true);
    expect(card?.classList.contains("bg-primary-100")).toBe(false);
    // the parts-count chip is a fill: dark-step background + white text
    const chip = screen.getByText("2");
    const chipRoot = chip.closest(".shrink-0");
    expect(chipRoot?.classList.contains("bg-secondary-700")).toBe(true);
    expect(chipRoot?.classList.contains("bg-secondary-100")).toBe(false);
    expect(chip.classList.contains("text-white")).toBe(true);
  });

  it("shows the admin role for an admin user", () => {
    renderCard({ user: { ...user, is_admin: true } });
    expect(screen.getByText("Admin")).toBeTruthy();
  });

  it("links the provider mark to the neutral user redirect route in a new tab", () => {
    const { utils } = renderCard();
    // The link is the provider-neutral redirect route — the provider is the
    // user's own, derived server-side. It opens in a new tab so the admin
    // page is not left.
    const link = utils.container.querySelector("a[href='/source/users/3']");
    expect(link).toBeTruthy();
    expect(link?.getAttribute("target")).toBe("_blank");
  });

  describe("action menu", () => {
    beforeEach(() => {
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
    });

    afterEach(() => {
      vi.useRealTimers();
    });

    /// Settle the dropdown. The Popper's trigger debounce (200ms, rAF) and
    /// the dropdown's fade transition (100ms, Web Animations API) each need
    /// timer advances, and Svelte schedules the animation on a microtask.
    async function settle() {
      for (let i = 0; i < 3; i++) {
        await vi.advanceTimersByTimeAsync(300);
        flushSync();
      }
    }

    /// The Popper's trigger is the menu's dots icon and listens for mousedown.
    async function openMenu(container: HTMLElement) {
      const trigger = container.querySelector("svg.cursor-pointer");
      expect(trigger).toBeTruthy();
      fireEvent.mouseDown(trigger!);
      await settle();
    }

    it("offers all four actions for an enabled user", async () => {
      const { utils } = renderCard();
      await openMenu(utils.container);
      expect(screen.getByText("Add Sync Event")).toBeTruthy();
      expect(screen.getByText("Process Event Queue")).toBeTruthy();
      expect(screen.getByText("Disable user")).toBeTruthy();
      expect(screen.getByText("Delete User")).toBeTruthy();
    });

    it("offers only delete for a disabled user", async () => {
      const { utils } = renderCard({ disabled: true });
      await openMenu(utils.container);
      expect(screen.getByText("Delete User")).toBeTruthy();
      expect(screen.queryByText("Add Sync Event")).toBeNull();
      expect(screen.queryByText("Process Event Queue")).toBeNull();
      expect(screen.queryByText("Disable user")).toBeNull();
    });

    it("opens the create-sync modal when add sync event is clicked", async () => {
      const { utils, createSync } = renderCard();
      await openMenu(utils.container);
      fireEvent.click(screen.getByText("Add Sync Event"));
      expect(createSync).toHaveBeenCalledWith(user);
    });

    it("opens the delete modal when delete user is clicked", async () => {
      const { utils, deleteUser } = renderCard();
      await openMenu(utils.container);
      fireEvent.click(screen.getByText("Delete User"));
      expect(deleteUser).toHaveBeenCalledWith(user);
    });

    it("fires the disable fetch and refreshes when disable user is clicked", async () => {
      const { utils, refresh } = renderCard();
      await openMenu(utils.container);
      fireEvent.click(screen.getByText("Disable user"));
      await settle();
      expect(fetchMock).toHaveBeenCalledWith(
        "/admin/disable/3",
        expect.objectContaining({ method: "POST" }),
      );
      expect(refresh).toHaveBeenCalled();
    });

    it("fires the process-queue fetch and refreshes when done", async () => {
      const { utils, refresh } = renderCard();
      await openMenu(utils.container);
      fireEvent.click(screen.getByText("Process Event Queue"));
      await settle();
      expect(fetchMock).toHaveBeenCalledWith("/admin/sync/3", undefined);
      expect(refresh).toHaveBeenCalled();
    });

    it("shows a spinner in the card header while the queue is processing", async () => {
      let resolveSync: () => void;
      fetchMock.mockImplementation((url: string) => {
        if (url.includes("/admin/sync/")) {
          return new Promise((resolve) => {
            resolveSync = () => resolve(resp(null, 204, true, ""));
          });
        }
        return Promise.resolve(resp(null, 204, true, ""));
      });
      const { utils, refresh } = renderCard();
      await openMenu(utils.container);
      fireEvent.click(screen.getByText("Process Event Queue"));
      await settle();
      // the menu trigger is swapped for a spinner while the queue drains
      expect(utils.container.querySelector("svg[role='status']")).toBeTruthy();
      expect(utils.container.querySelector("svg.cursor-pointer")).toBeNull();
      resolveSync!();
      await settle();
      expect(utils.container.querySelector("svg[role='status']")).toBeNull();
      expect(refresh).toHaveBeenCalled();
    });
  });
});
