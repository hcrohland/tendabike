import { render, screen } from "@testing-library/svelte";
import { flushSync } from "svelte";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { activities, Activity } from "./lib/activity";
import { parts, Part } from "./lib/part";
import { Shop, setShop } from "./lib/shop";
import { getTypes, setCategory, types } from "./lib/types";
import { setUser } from "./lib/user";
import { resp } from "./test/helpers";
import ToyGroup from "./ToyGroup.svelte";

describe("ToyGroup", () => {
  let fetchMock: ReturnType<typeof vi.fn>;

  beforeEach(async () => {
    fetchMock = vi.fn().mockImplementation((url: string) => {
      if (url.includes("/api/types/part")) {
        return Promise.resolve(
          resp([
            { id: 1, name: "Tire", main: 1, hooks: [1], order: 1 },
            { id: 2, name: "Brake", main: 2, hooks: [1], order: 2 },
          ]),
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
    });
    parts.setMap([]);
    activities.setMap([]);
    setShop(undefined);
  });

  it("renders parts matching the category", () => {
    parts.setMap([
      new Part({
        id: 1,
        owner: 1,
        what: 1,
        name: "Front Tire",
        last_used: new Date("2024-01-01"),
      }),
      new Part({
        id: 2,
        owner: 1,
        what: 2,
        name: "Brake",
        last_used: new Date("2024-01-01"),
      }),
    ]);
    render(ToyGroup);
    expect(screen.getByText("Front Tire")).toBeTruthy();
    expect(screen.queryByText("Brake")).toBeNull();
  });

  it("re-renders when the current shop changes", () => {
    parts.setMap([
      new Part({
        id: 1,
        owner: 1,
        what: 1,
        name: "Shop Tire",
        shop: 7,
        last_used: new Date("2024-01-01"),
      }),
      new Part({
        id: 2,
        owner: 1,
        what: 1,
        name: "Home Tire",
        last_used: new Date("2024-01-01"),
      }),
    ]);
    const { unmount } = render(ToyGroup);
    expect(screen.getByText("Shop Tire")).toBeTruthy();
    expect(screen.getByText("Home Tire")).toBeTruthy();
    setShop(
      new Shop({
        id: 7,
        owner: 2,
        name: "Velo Shop",
        auto_approve: false,
        created_at: "2023-01-01T00:00:00Z",
      }),
    );
    flushSync();
    expect(screen.getByText("Shop Tire")).toBeTruthy();
    expect(screen.queryByText("Home Tire")).toBeNull();
    setShop(undefined);
    flushSync();
    expect(screen.getByText("Home Tire")).toBeTruthy();
    unmount();
  });

  it("re-renders when the category changes", () => {
    parts.setMap([
      new Part({
        id: 1,
        owner: 1,
        what: 1,
        name: "Tire Part",
        last_used: new Date("2024-01-01"),
      }),
      new Part({
        id: 2,
        owner: 1,
        what: 2,
        name: "Brake Part",
        last_used: new Date("2024-01-01"),
      }),
    ]);
    const { unmount } = render(ToyGroup);
    expect(screen.getByText("Tire Part")).toBeTruthy();
    expect(screen.queryByText("Brake Part")).toBeNull();
    setCategory(types[2]);
    flushSync();
    expect(screen.queryByText("Tire Part")).toBeNull();
    expect(screen.getByText("Brake Part")).toBeTruthy();
    setCategory(types[1]);
    flushSync();
    expect(screen.getByText("Tire Part")).toBeTruthy();
    expect(screen.queryByText("Brake Part")).toBeNull();
    unmount();
  });

  it("re-renders when the collection changes", () => {
    const { unmount } = render(ToyGroup);
    expect(screen.queryByText("Front Tire")).toBeNull();
    parts.updateMap([
      new Part({
        id: 1,
        owner: 1,
        what: 1,
        name: "Front Tire",
        last_used: new Date("2024-01-01"),
      }),
    ]);
    flushSync();
    expect(screen.getByText("Front Tire")).toBeTruthy();
    unmount();
  });

  it("shows 'none found' message when no parts and no activities", () => {
    render(ToyGroup);
    expect(
      screen.getByText("We did not find any Tire on Strava (yet)."),
    ).toBeTruthy();
  });

  it("shows 'none assigned' message when no parts but activities exist", () => {
    activities.setMap([
      new Activity({
        id: 1,
        user_id: 1,
        what: 1,
        name: "Ride",
        start: new Date("2024-01-01"),
      }),
    ]);
    render(ToyGroup);
    expect(
      screen.getByText(
        "You have no Tire assigned to any activity on Strava. Please do so to get started.",
      ),
    ).toBeTruthy();
  });

  it("re-renders when the activities collection changes", () => {
    const { unmount } = render(ToyGroup);
    expect(
      screen.getByText("We did not find any Tire on Strava (yet)."),
    ).toBeTruthy();
    activities.setMap([
      new Activity({
        id: 1,
        user_id: 1,
        what: 1,
        name: "Ride",
        start: new Date("2024-01-01"),
      }),
    ]);
    flushSync();
    expect(
      screen.getByText(
        "You have no Tire assigned to any activity on Strava. Please do so to get started.",
      ),
    ).toBeTruthy();
    unmount();
  });

  it("renders ShowMore section when disposed parts exist", async () => {
    parts.setMap([
      new Part({
        id: 1,
        owner: 1,
        what: 1,
        name: "Good Tire",
        last_used: new Date("2024-01-01"),
      }),
      new Part({
        id: 2,
        owner: 1,
        what: 1,
        name: "Old Tire",
        last_used: new Date("2024-01-01"),
        disposed_at: new Date("2024-06-01"),
      }),
    ]);
    const { container } = render(ToyGroup);
    expect(screen.getByText("Good Tire")).toBeTruthy();
    expect(container.querySelector("button")).toBeTruthy();
  });
});
