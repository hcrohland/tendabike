import { fireEvent, render, screen } from "@testing-library/svelte";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { setUser } from "../lib/user.svelte";
import { resp } from "../test/helpers";
import InitialSyncDialog from "./InitialSyncDialog.svelte";

describe("InitialSyncDialog", () => {
  let fetchMock: ReturnType<typeof vi.fn>;

  function pendingUser(provider: "strava" | "icu") {
    return {
      id: 1,
      firstname: "Max",
      name: "Mustermann",
      avatar: undefined,
      is_admin: false,
      onboarding_status: "pending" as const,
      provider,
    };
  }

  beforeEach(() => {
    fetchMock = vi.fn().mockImplementation((url: string) => {
      if (url.includes("/onboarding/gear")) {
        // The onboarding gear import's response (spec #829, the first step):
        // the gear items the provider import discovered.
        return Promise.resolve(
          resp({
            gear: [
              { name: "Canyon Aeroad", what: 1 },
              { name: "Shimano S-Phyre RC2", what: 301 },
            ],
          }),
        );
      }
      if (url.includes("/onboarding/sync")) {
        // The onboarding activity sync's response (spec #829, the second
        // step): the updated user.
        return Promise.resolve(
          resp({
            ...pendingUser("strava"),
            onboarding_status: "completed",
          }),
        );
      }
      if (url.includes("/onboarding/postpone")) {
        // The postpone's response: the updated user.
        return Promise.resolve(
          resp({
            ...pendingUser("strava"),
            onboarding_status: "initial_sync_postponed",
          }),
        );
      }
      return Promise.resolve(resp(null));
    });
    vi.stubGlobal("fetch", fetchMock);
  });

  it("opens branded for a pending Strava user, with the skip option", async () => {
    setUser(pendingUser("strava"));
    render(InitialSyncDialog);
    expect(await screen.findByText("Import from Strava")).toBeTruthy();
    expect(screen.getByRole("button", { name: "Skip for Now" })).toBeTruthy();
  });

  it("opens branded for a pending intervals.icu user, with the skip option", async () => {
    setUser(pendingUser("icu"));
    render(InitialSyncDialog);
    expect(await screen.findByText("Import from intervals.icu")).toBeTruthy();
    expect(screen.getByRole("button", { name: "Skip for Now" })).toBeTruthy();
  });

  it("imports the gear on open (Strava)", async () => {
    setUser(pendingUser("strava"));
    render(InitialSyncDialog);
    // The gear import (Step 1) fires on dialog open.
    await vi.waitFor(() => {
      expect(fetchMock).toHaveBeenCalledWith(
        "/strava/onboarding/gear",
        expect.objectContaining({ method: "POST" }),
      );
    });
    // The discovered gear is rendered as chips.
    expect(await screen.findByText("Canyon Aeroad")).toBeTruthy();
    expect(screen.getByText("Shimano S-Phyre RC2")).toBeTruthy();
  });

  it("imports the gear on open (icu)", async () => {
    setUser(pendingUser("icu"));
    render(InitialSyncDialog);
    await vi.waitFor(() => {
      expect(fetchMock).toHaveBeenCalledWith(
        "/icu/onboarding/gear",
        expect.objectContaining({ method: "POST" }),
      );
    });
  });

  it("triggers the Strava activity sync on Import Activities", async () => {
    setUser(pendingUser("strava"));
    render(InitialSyncDialog);
    // Wait for the gear import (Step 1) to land so the CTA is enabled.
    await screen.findByText("Canyon Aeroad");
    fireEvent.click(
      await screen.findByRole("button", { name: "Import Activities" }),
    );
    await vi.waitFor(() => {
      expect(fetchMock).toHaveBeenCalledWith(
        "/strava/onboarding/sync",
        expect.objectContaining({ method: "POST" }),
      );
    });
  });

  it("triggers the icu activity sync on Import Activities", async () => {
    setUser(pendingUser("icu"));
    render(InitialSyncDialog);
    await screen.findByText("Canyon Aeroad");
    fireEvent.click(
      await screen.findByRole("button", { name: "Import Activities" }),
    );
    await vi.waitFor(() => {
      expect(fetchMock).toHaveBeenCalledWith(
        "/icu/onboarding/sync",
        expect.objectContaining({ method: "POST" }),
      );
    });
  });

  it("postpones on Skip for now", async () => {
    setUser(pendingUser("strava"));
    render(InitialSyncDialog);
    fireEvent.click(screen.getByRole("button", { name: "Skip for Now" }));
    await vi.waitFor(() => {
      expect(fetchMock).toHaveBeenCalledWith(
        "/strava/onboarding/postpone",
        expect.objectContaining({ method: "POST" }),
      );
    });
  });

  it("postpones on Skip for now (icu)", async () => {
    setUser(pendingUser("icu"));
    render(InitialSyncDialog);
    fireEvent.click(screen.getByRole("button", { name: "Skip for Now" }));
    await vi.waitFor(() => {
      expect(fetchMock).toHaveBeenCalledWith(
        "/icu/onboarding/postpone",
        expect.objectContaining({ method: "POST" }),
      );
    });
  });

  it("postpones and closes on ESC (strava)", async () => {
    setUser(pendingUser("strava"));
    render(InitialSyncDialog);
    // Wait for the gear import (Step 1) to land so the dialog is idle.
    await screen.findByText("Canyon Aeroad");
    const dialog = screen.getByRole("dialog");
    // ESC on a modal dialog: the browser fires `cancel` on the dialog
    // element — the boundary the dialog hooks (spec #974).
    dialog.dispatchEvent(
      new Event("cancel", { bubbles: true, cancelable: true }),
    );
    await vi.waitFor(() => {
      expect(fetchMock).toHaveBeenCalledWith(
        "/strava/onboarding/postpone",
        expect.objectContaining({ method: "POST" }),
      );
    });
    // The dialog closes once the postponement lands.
    await vi.waitFor(() => {
      expect(screen.queryByRole("dialog")).toBeNull();
    });
  });

  it("postpones and closes on ESC (icu)", async () => {
    setUser(pendingUser("icu"));
    render(InitialSyncDialog);
    await screen.findByText("Canyon Aeroad");
    const dialog = screen.getByRole("dialog");
    dialog.dispatchEvent(
      new Event("cancel", { bubbles: true, cancelable: true }),
    );
    await vi.waitFor(() => {
      expect(fetchMock).toHaveBeenCalledWith(
        "/icu/onboarding/postpone",
        expect.objectContaining({ method: "POST" }),
      );
    });
    await vi.waitFor(() => {
      expect(screen.queryByRole("dialog")).toBeNull();
    });
  });

  it("renders each gear chip with its type icon", async () => {
    setUser(pendingUser("strava"));
    render(InitialSyncDialog);
    const bikeChip = (await screen.findByText("Canyon Aeroad")).closest("span");
    expect(bikeChip?.querySelector("i")?.className).toBe(
      "flaticon-mountain-bike",
    );
    const shoeChip = screen.getByText("Shimano S-Phyre RC2").closest("span");
    expect(shoeChip?.querySelector("i")?.className).toBe("flaticon-run");
  });
});
