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
        return Promise.resolve(
          resp({ ...pendingUser("strava"), onboarding_status: "completed" }),
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
});
