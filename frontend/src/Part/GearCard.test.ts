import { render, screen } from "@testing-library/svelte";
import { flushSync } from "svelte";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { Part } from "../lib/part";
import { getTypes } from "../lib/types.svelte";
import { setUser, users } from "../lib/user.svelte";
import { resp } from "../test/helpers";
import GearCard from "./GearCard.svelte";

describe("GearCard", () => {
  let fetchMock: ReturnType<typeof vi.fn>;

  beforeEach(async () => {
    fetchMock = vi.fn().mockImplementation((url: string) => {
      if (url.includes("/api/types/part")) {
        return Promise.resolve(
          resp([{ id: 1, name: "Bike", main: 1, hooks: [1], order: 1 }]),
        );
      }
      if (url.includes("/api/types/activity")) {
        return Promise.resolve(resp([{ id: 1, name: "Ride", gear_type: 1 }]));
      }
      return Promise.resolve(resp(null));
    });
    vi.stubGlobal("fetch", fetchMock);
    await getTypes();

    setUser({
      id: 1,
      firstname: "Test",
      name: "Test User",
      avatar: undefined,
      is_admin: false,
      onboarding_status: "completed",
      provider: "strava",
    });
    users.setMap([]);
  });

  const part = new Part({
    id: 7,
    owner: 5,
    what: 1,
    name: "My Bike",
    last_used: new Date("2024-01-01"),
  });

  it("shows the owner badge for a part owned by another user", () => {
    users.setMap([
      { id: 5, firstname: "Max", name: "Mustermann", avatar: undefined },
    ]);
    render(GearCard, { part });
    expect(screen.getByText("Max Mustermann")).toBeTruthy();
  });

  it("re-renders the owner badge when the users collection changes", () => {
    users.setMap([
      { id: 5, firstname: "Max", name: "Mustermann", avatar: undefined },
    ]);
    const { unmount } = render(GearCard, { part });
    expect(screen.getByText("Max Mustermann")).toBeTruthy();
    users.updateMap([
      { id: 5, firstname: "Erin", name: "Brooks", avatar: undefined },
    ]);
    flushSync();
    expect(screen.getByText("Erin Brooks")).toBeTruthy();
    expect(screen.queryByText("Max Mustermann")).toBeNull();
    unmount();
  });

  it("re-renders the owner badge when the current user changes", () => {
    users.setMap([
      { id: 5, firstname: "Max", name: "Mustermann", avatar: undefined },
    ]);
    const { unmount } = render(GearCard, { part });
    expect(screen.getByText("Max Mustermann")).toBeTruthy();
    setUser({
      id: 5,
      firstname: "Max",
      name: "Mustermann",
      avatar: undefined,
      is_admin: false,
      onboarding_status: "completed",
      provider: "strava",
    });
    flushSync();
    expect(screen.queryByText("Max Mustermann")).toBeNull();
    unmount();
  });

  it("centers the provider mark in the meta line (no baseline hang)", () => {
    users.setMap([
      { id: 5, firstname: "Max", name: "Mustermann", avatar: undefined },
    ]);
    const sourced = new Part({
      id: 8,
      owner: 5,
      what: 1,
      name: "My Bike",
      last_used: new Date("2024-01-01"),
      source: "icu:123",
    });
    const { container } = render(GearCard, { part: sourced });
    const mark = [...container.querySelectorAll("span")].find(
      (s) => s.textContent === "icu",
    );
    const link = mark?.closest("a");
    // The mark sits in a centered flex, not a vertical-align:middle inline —
    // the latter hangs a tall badge below the text line.
    expect(link?.className).toContain("items-center");
  });
});
