import { fireEvent, render } from "@testing-library/svelte";
import { flushSync } from "svelte";
import { beforeEach, describe, expect, it } from "vitest";
import { Part, parts } from "../lib/part";
import { Service, services } from "../lib/service";
import { usages } from "../lib/usage";
import { expectSurface, partData, standardUsage } from "../test/helpers";
import ServiceHist from "./ServiceHist.svelte";

describe("ServiceHist", () => {
  beforeEach(() => {
    parts.setMap([]);
    usages.setMap([]);
    services.setMap([]);
  });

  function seed() {
    const part = new Part(partData());
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
    usages.setMap([standardUsage()]);
    return { part, service };
  }

  it("renders the card's secondary surface", () => {
    const { service } = seed();
    const { container } = render(ServiceHist, { service });
    expectSurface(container.querySelector(".rounded-lg"), "secondary");
  });

  it("renders the service history on the contrasting (primary) surface", () => {
    const { service } = seed();
    const { container } = render(ServiceHist, { service });
    const showMore = container.querySelector("button.rounded-full");
    expect(showMore).toBeTruthy();
    fireEvent.click(showMore!);
    flushSync();
    // the nested history block takes the contrasting primary surface
    const block = container.querySelector(".surface-primary");
    expect(block).toBeTruthy();
  });
});
