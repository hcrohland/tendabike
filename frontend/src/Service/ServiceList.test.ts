import { render, screen } from "@testing-library/svelte";
import { flushSync } from "svelte";
import { beforeEach, describe, expect, it } from "vitest";
import { parts, Part } from "../lib/part";
import { services, Service } from "../lib/service";
import { usages, Usage } from "../lib/usage";
import ServiceList from "./ServiceList.svelte";

describe("ServiceList", () => {
  beforeEach(() => {
    parts.setMap([]);
    usages.setMap([]);
    services.setMap([]);
  });

  function seedPart() {
    const part = new Part({
      id: 7,
      owner: 1,
      what: 10,
      name: "Wheel",
      purchase: "2023-01-01T00:00:00Z",
      last_used: "2024-01-01T00:00:00Z",
      usage: "u1",
    });
    parts.setMap([part]);
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
    return part;
  }

  it("re-renders when the services collection changes", () => {
    const part = seedPart();
    const { unmount } = render(ServiceList, { part });
    expect(screen.queryByText("Annual Service")).toBeNull();
    services.updateMap([
      new Service({
        id: "S1",
        part_id: 7,
        time: "2024-01-01T00:00:00Z",
        redone: "2023-01-01T00:00:00Z",
        name: "Annual Service",
        notes: "",
        usage: "u1",
        successor: null,
        plans: [],
      }),
    ]);
    flushSync();
    expect(screen.getByText("Annual Service")).toBeTruthy();
    unmount();
  });
});
