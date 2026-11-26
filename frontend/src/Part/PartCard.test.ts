import { fireEvent, render } from "@testing-library/svelte";
import { flushSync } from "svelte";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { Attachment } from "../lib/attachment";
import { Part, parts } from "../lib/part";
import { getTypes, types } from "../lib/types.svelte";
import { usages } from "../lib/usage";
import { resp, usage } from "../test/helpers";
import PartCard from "./PartCard.svelte";

// The part card is a migrated call site: its surface is an explicit `color`
// (default secondary), and its nested blocks — the history entries and the
// child cards — take the opposite color. We assert only the rendered surface
// classes, never the mechanism.
describe("PartCard", () => {
  let fetchMock: ReturnType<typeof vi.fn>;

  beforeEach(async () => {
    fetchMock = vi.fn().mockImplementation((url: string) => {
      if (url.includes("/api/types/part")) {
        return Promise.resolve(
          resp([{ id: 10, name: "Wheel", main: 1, hooks: [1], order: 1 }]),
        );
      }
      if (url.includes("/api/types/activity")) {
        return Promise.resolve(resp([]));
      }
      return Promise.resolve(resp(null));
    });
    vi.stubGlobal("fetch", fetchMock);
    await getTypes();
    parts.setMap([]);
    usages.setMap([]);
  });

  function att(
    partId: number,
    gear: number,
    name: string,
    attached = "2024-01-01T00:00:00Z",
    detached = "2999-01-01T00:00:00Z",
  ) {
    return new Attachment({
      part_id: partId,
      attached,
      gear,
      hook: 1,
      detached,
      what: 10,
      name,
      usage: "u1",
    });
  }

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
    const child = new Part({
      id: 8,
      owner: 1,
      what: 10,
      name: "Inner Wheel",
      purchase: "2023-01-01T00:00:00Z",
      last_used: "2024-01-01T00:00:00Z",
      usage: "u1",
    });
    parts.setMap([part, child]);
    usages.setMap([
      usage("u1", {
        count: 1,
        distance: 1000,
        time: 3600,
        duration: 3600,
        energy: 100,
      }),
    ]);
    return { part, child };
  }

  it("renders the card's secondary surface by default", () => {
    seed();
    const { container } = render(PartCard, {
      attachments: [att(7, 9, "Wheel")],
      type: types[10],
      children: [],
    });
    const card = container.querySelector(".rounded-lg");
    expect(card?.classList.contains("bg-secondary-100")).toBe(true);
    expect(card?.classList.contains("bg-primary-100")).toBe(false);
  });

  it("renders a nested child card on the contrasting (primary) surface", () => {
    seed();
    const { container } = render(PartCard, {
      attachments: [att(7, 9, "Wheel")],
      type: types[10],
      children: [
        {
          attachments: [att(8, 7, "Inner Wheel")],
          type: types[10],
          children: [],
        },
      ],
    });
    const childCard = container.querySelector(".mt-3 .rounded-lg");
    expect(childCard?.classList.contains("bg-primary-100")).toBe(true);
    expect(childCard?.classList.contains("bg-secondary-100")).toBe(false);
  });

  it("renders the primary surface with a secondary child when given color=primary", () => {
    seed();
    const { container } = render(PartCard, {
      attachments: [att(7, 9, "Wheel")],
      type: types[10],
      children: [
        {
          attachments: [att(8, 7, "Inner Wheel")],
          type: types[10],
          children: [],
        },
      ],
      color: "primary",
    });
    const card = container.querySelector(".rounded-lg");
    expect(card?.classList.contains("bg-primary-100")).toBe(true);
    const childCard = container.querySelector(".mt-3 .rounded-lg");
    expect(childCard?.classList.contains("bg-secondary-100")).toBe(true);
    expect(childCard?.classList.contains("bg-primary-100")).toBe(false);
  });

  it("renders the part history block on the contrasting (primary) surface", () => {
    seed();
    const { container } = render(PartCard, {
      attachments: [
        att(7, 9, "Wheel"),
        att(7, 9, "Wheel", "2020-01-01T00:00:00Z", "2023-01-01T00:00:00Z"),
      ],
      type: types[10],
      children: [],
    });
    const showMore = container.querySelector("button.rounded-full");
    expect(showMore).toBeTruthy();
    fireEvent.click(showMore!);
    flushSync();
    const block = container.querySelector(".opacity-70");
    expect(block?.classList.contains("bg-primary-100")).toBe(true);
    expect(block?.classList.contains("bg-secondary-100")).toBe(false);
  });
});
