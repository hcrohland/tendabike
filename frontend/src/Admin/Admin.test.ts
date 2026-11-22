import { render, screen } from "@testing-library/svelte";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { resp } from "../test/helpers";
import Admin from "./Admin.svelte";

const list = [
  {
    user: { id: 1, firstname: "Max", name: "Mustermann" },
    parts: 1,
    activities: 2,
    events: 3,
    disabled: false,
    provider: "strava",
  },
  {
    user: { id: 2, firstname: "Erin", name: "Brooks" },
    parts: 0,
    activities: 0,
    events: 0,
    disabled: true,
    provider: "icu",
  },
];

describe("Admin", () => {
  let fetchMock: ReturnType<typeof vi.fn>;

  beforeEach(() => {
    fetchMock = vi.fn().mockImplementation((url: string) => {
      if (url.includes("/api/user/all")) {
        return Promise.resolve(resp(list));
      }
      return Promise.resolve(resp(null, 204, true, ""));
    });
    vi.stubGlobal("fetch", fetchMock);
  });

  it("renders one card per user without a table", async () => {
    const { container } = render(Admin);
    expect(await screen.findByText("Max Mustermann")).toBeTruthy();
    expect(screen.getByText("Erin Brooks")).toBeTruthy();
    expect(container.querySelector("table")).toBeNull();
  });

  it("keeps the page-level buttons", async () => {
    render(Admin);
    expect(await screen.findByText("Add Sync Event for all")).toBeTruthy();
    expect(screen.getByText("Rescan all activities")).toBeTruthy();
    expect(screen.getByText("Refresh")).toBeTruthy();
  });
});
