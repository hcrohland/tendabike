import { render, screen } from "@testing-library/svelte";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { activities, Activity } from "./lib/activity";
import { parts, Part } from "./lib/part";
import { shop } from "./lib/shop";
import { getTypes } from "./lib/types";
import { user } from "./lib/user";
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

    user.set({
      id: 1,
      firstname: "Test",
      name: "Test User",
      avatar: undefined,
      is_admin: false,
      onboarding_status: "completed",
    });
    parts.setMap([]);
    activities.setMap([]);
    shop.set(undefined);
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
