import { fireEvent, render, screen } from "@testing-library/svelte";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { setUser } from "../lib/user";
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
      if (url.includes("/onboarding/sync")) {
        // The onboarding sync response (spec #829): the updated user plus the
        // gear items the provider import discovered.
        return Promise.resolve(
          resp({
            user: {
              ...pendingUser("strava"),
              onboarding_status: "completed",
            },
            gear: [
              { name: "Canyon Aeroad", what: 1 },
              { name: "Shimano S-Phyre RC2", what: 301 },
            ],
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

  it("opens branded for a pending intervals.icu user, without the skip option", async () => {
    setUser(pendingUser("icu"));
    render(InitialSyncDialog);
    expect(await screen.findByText("Import from intervals.icu")).toBeTruthy();
    expect(screen.queryByRole("button", { name: "Skip for Now" })).toBeNull();
  });

  it("triggers the Strava onboarding sync", async () => {
    setUser(pendingUser("strava"));
    render(InitialSyncDialog);
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

  it("triggers the icu onboarding sync", async () => {
    setUser(pendingUser("icu"));
    render(InitialSyncDialog);
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

  it("renders the discovered gear as chips after the onboarding sync", async () => {
    setUser(pendingUser("strava"));
    render(InitialSyncDialog);
    fireEvent.click(
      await screen.findByRole("button", { name: "Import Activities" }),
    );
    expect(await screen.findByText("Canyon Aeroad")).toBeTruthy();
    expect(screen.getByText("Shimano S-Phyre RC2")).toBeTruthy();
  });

  it("renders each gear chip with its type icon", async () => {
    setUser(pendingUser("strava"));
    render(InitialSyncDialog);
    fireEvent.click(
      await screen.findByRole("button", { name: "Import Activities" }),
    );
    const bikeChip = (await screen.findByText("Canyon Aeroad")).closest("span");
    expect(bikeChip?.querySelector("i")?.className).toBe(
      "flaticon-mountain-bike",
    );
    const shoeChip = screen.getByText("Shimano S-Phyre RC2").closest("span");
    expect(shoeChip?.querySelector("i")?.className).toBe("flaticon-run");
  });

  it("offers a close action once the gear is discovered", async () => {
    setUser(pendingUser("strava"));
    render(InitialSyncDialog);
    fireEvent.click(
      await screen.findByRole("button", { name: "Import Activities" }),
    );
    await screen.findByText("Canyon Aeroad");
    expect(screen.getByRole("button", { name: "Close" })).toBeTruthy();
  });
});
