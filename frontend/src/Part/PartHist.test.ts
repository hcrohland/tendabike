import { render } from "@testing-library/svelte";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { Attachment, attachments } from "../lib/attachment";
import { Part, parts } from "../lib/part";
import { getTypes } from "../lib/types.svelte";
import { usages } from "../lib/usage";
import { resp, usage } from "../test/helpers";
import PartHist from "./PartHist.svelte";

// The part history section is a migrated call site: its outer block renders
// the primary surface and the nested attachment cards take the contrasting
// secondary surface. We assert only the rendered surface classes, never the
// mechanism.
describe("PartHist", () => {
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
    attachments.setMap([]);
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
    parts.setMap([part]);
    usages.setMap([
      usage("u1", {
        count: 1,
        distance: 1000,
        time: 3600,
        duration: 3600,
        energy: 100,
      }),
    ]);
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
    const outer = container.querySelector(".m-2");
    expect(outer?.classList.contains("bg-primary-100")).toBe(true);
    expect(outer?.classList.contains("bg-secondary-100")).toBe(false);
  });

  it("renders the nested attachment cards on the contrasting (secondary) surface", () => {
    seed();
    const { container } = render(PartHist, { id: 7 });
    const inner = container.querySelector(".m-2 .rounded-lg");
    expect(inner?.classList.contains("bg-secondary-100")).toBe(true);
    expect(inner?.classList.contains("bg-primary-100")).toBe(false);
  });
});
