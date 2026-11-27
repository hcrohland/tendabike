import { fireEvent, render } from "@testing-library/svelte";
import { flushSync } from "svelte";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { Attachment } from "../lib/attachment";
import { Part, parts } from "../lib/part";
import { getTypes, types } from "../lib/types.svelte";
import { usages } from "../lib/usage";
import {
  expectSurface,
  partData,
  standardUsage,
  typesFetch,
} from "../test/helpers";
import PartCard from "./PartCard.svelte";

describe("PartCard", () => {
  let fetchMock: ReturnType<typeof typesFetch>;

  beforeEach(async () => {
    fetchMock = typesFetch();
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
    const part = new Part(partData());
    const child = new Part(partData({ id: 8, name: "Inner Wheel" }));
    parts.setMap([part, child]);
    usages.setMap([standardUsage()]);
    return { part, child };
  }

  it("renders the card's secondary surface by default", () => {
    seed();
    const { container } = render(PartCard, {
      attachments: [att(7, 9, "Wheel")],
      type: types[10],
      children: [],
    });
    expectSurface(container.querySelector(".rounded-lg"), "secondary");
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
    expectSurface(container.querySelector(".mt-3 .rounded-lg"), "primary");
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
    expectSurface(container.querySelector(".rounded-lg"), "primary");
    expectSurface(container.querySelector(".mt-3 .rounded-lg"), "secondary");
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
    expectSurface(container.querySelector(".opacity-70"), "primary");
  });
});
