import { fireEvent, render } from "@testing-library/svelte";
import { beforeEach, describe, expect, it } from "vitest";
import { Part, parts } from "../lib/part";
import { Service, services } from "../lib/service";
import { usages } from "../lib/usage";
import { usage } from "../test/helpers";
import ServiceHist from "./ServiceHist.svelte";

// The service history card is a migrated call site: its surface is an explicit
// secondary color, and its nested history block takes the contrasting primary
// surface. We assert only the rendered surface classes, never the mechanism.
describe("ServiceHist", () => {
  beforeEach(() => {
    parts.setMap([]);
    usages.setMap([]);
    services.setMap([]);
  });

  function seed() {
    const part = new Part({
      id: 7,
      owner: 1,
      what: 10,
      name: "Wheel",
      purchase: "2023-01-01T00:00:00Z",
      last_used: "2024-01-01T00:00:00Z",
      usage: "u1",
    });
    const service = new Service({
      id: "S1",
      part_id: 7,
      time: "2024-01-01T00:00:00Z",
      redone: "2023-01-01T00:00:00Z",
      name: "Annual Service",
      notes: "",
      usage: "u1",
      successor: null,
      plans: [],
    });
    const pred = new Service({
      id: "S0",
      part_id: 7,
      time: "2023-06-01T00:00:00Z",
      redone: "2023-01-01T00:00:00Z",
      name: "Earlier Service",
      notes: "",
      usage: "u1",
      successor: "S1",
      plans: [],
    });
    parts.setMap([part]);
    services.setMap([service, pred]);
    usages.setMap([
      usage("u1", {
        count: 1,
        distance: 1000,
        time: 3600,
        duration: 3600,
        energy: 100,
      }),
    ]);
    return { part, service };
  }

  it("renders the card's secondary surface", () => {
    const { service } = seed();
    const { container } = render(ServiceHist, { service });
    const card = container.querySelector(".rounded-lg");
    expect(card?.classList.contains("bg-secondary-100")).toBe(true);
    expect(card?.classList.contains("bg-primary-100")).toBe(false);
  });

  it("renders the service history on the contrasting (primary) surface", () => {
    const { service } = seed();
    const { container } = render(ServiceHist, { service });
    const showMore = container.querySelector("button.rounded-full");
    expect(showMore).toBeTruthy();
    fireEvent.click(showMore!);
    const block = container.querySelector(".bg-primary-100");
    expect(block).toBeTruthy();
  });
});
