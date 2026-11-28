import { render } from "@testing-library/svelte";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { Attachment, attachments } from "../lib/attachment";
import { Part, parts } from "../lib/part";
import { getTypes } from "../lib/types.svelte";
import { usages } from "../lib/usage";
import {
  expectSurface,
  partData,
  standardUsage,
  typesFetch,
} from "../test/helpers";
import PartHist from "./PartHist.svelte";

describe("PartHist", () => {
  let fetchMock: ReturnType<typeof typesFetch>;

  beforeEach(async () => {
    fetchMock = typesFetch();
    vi.stubGlobal("fetch", fetchMock);
    await getTypes();
    parts.setMap([]);
    usages.setMap([]);
    attachments.setMap([]);
  });

  function seed() {
    const part = new Part(partData());
    parts.setMap([part]);
    usages.setMap([standardUsage()]);
    attachments.setMap([
      new Attachment({
        part_id: 7,
        attached: "2024-01-01T00:00:00Z",
        gear: 9,
        hook: 1,
        detached: "2999-01-01T00:00:00Z",
        what: 10,
        name: "Wheel",
        usage: "u1",
      }),
      new Attachment({
        part_id: 7,
        attached: "2020-01-01T00:00:00Z",
        gear: 9,
        hook: 1,
        detached: "2023-01-01T00:00:00Z",
        what: 10,
        name: "Wheel",
        usage: "u1",
      }),
    ]);
    return part;
  }

  it("renders the history section on the primary surface", () => {
    seed();
    const { container } = render(PartHist, { id: 7 });
    expectSurface(container.querySelector(".m-2"), "primary");
  });

  it("renders the nested attachment cards on the contrasting (secondary) surface", () => {
    seed();
    const { container } = render(PartHist, { id: 7 });
    expectSurface(container.querySelector(".m-2 .rounded-lg"), "secondary");
  });
});
