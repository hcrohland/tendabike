import { render, screen } from "@testing-library/svelte";
import { flushSync } from "svelte";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { attachments } from "../lib/attachment";
import { parts, Part } from "../lib/part";
import { plans, ServicePlan } from "../lib/serviceplan";
import { getTypes } from "../lib/types";
import { Usage, usages } from "../lib/usage";
import { resp } from "../test/helpers";
import PlanList from "./PlanList.svelte";

describe("PlanList", () => {
  let fetchMock: ReturnType<typeof vi.fn>;

  beforeEach(async () => {
    parts.setMap([]);
    usages.setMap([]);
    attachments.setMap([]);
    plans.setMap([]);
    fetchMock = vi.fn().mockImplementation((url: string) => {
      if (url.includes("/api/types/part")) {
        return Promise.resolve(
          resp([
            { id: 1, name: "Bike", main: 1, hooks: [1], order: 1 },
            { id: 10, name: "Wheel", main: 1, hooks: [1], order: 2 },
          ]),
        );
      }
      if (url.includes("/api/types/activity")) {
        return Promise.resolve(resp([]));
      }
      return Promise.resolve(resp(null));
    });
    vi.stubGlobal("fetch", fetchMock);
    await getTypes();
  });

  it("re-renders when the plans collection changes", () => {
    const { unmount } = render(PlanList);
    expect(screen.queryByText(/^Wheel Service/)).toBeNull();
    parts.setMap([
      new Part({
        id: 5,
        owner: 1,
        what: 10,
        name: "Wheel",
        purchase: "2023-01-01T00:00:00Z",
        last_used: "2024-01-01T00:00:00Z",
        usage: "u1",
      }),
    ]);
    usages.setMap([
      new Usage({
        id: "u1",
        count: 1,
        climb: 0,
        descend: 0,
        distance: 1000,
        time: 3600,
        duration: 3600,
        energy: 100,
      }),
    ]);
    plans.updateMap([
      new ServicePlan({
        id: "P1",
        part: 5,
        what: 10,
        hook: null,
        name: "Wheel Service",
      }),
    ]);
    flushSync();
    expect(screen.getByText(/^Wheel Service/)).toBeTruthy();
    unmount();
  });
});
