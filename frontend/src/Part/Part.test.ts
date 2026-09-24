import {
  fireEvent,
  render,
  screen,
  waitFor,
  within,
} from "@testing-library/svelte";
import { flushSync } from "svelte";
import { get } from "svelte/store";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { Attachment, attachments } from "../lib/attachment";
import { maxDate } from "../lib/store";
import { Part, parts } from "../lib/part";
import { partNotes } from "../lib/partnote";
import { getTypes } from "../lib/types";
import { setUser } from "../lib/user";
import { actions } from "../Widgets/Actions.svelte";
import { resp } from "../test/helpers";
import PartComponent from "./Part.svelte";

describe("Part", () => {
  let fetchMock: ReturnType<typeof vi.fn>;

  beforeEach(async () => {
    fetchMock = vi.fn().mockImplementation((url: string) => {
      if (url.includes("/api/types/part")) {
        return Promise.resolve(
          resp([
            { id: 1, name: "Bike", main: 1, hooks: [1], order: 1 },
            { id: 2, name: "Chain", main: 1, hooks: [1], order: 2 },
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

    setUser({
      id: 1,
      firstname: "Test",
      name: "Test User",
      avatar: undefined,
      is_admin: false,
      onboarding_status: "completed",
    });
    partNotes.setMap([]);
    attachments.setMap([]);
    actions.set({ newNote: vi.fn() } as never);
  });

  function seedPart(what: number) {
    const part = new Part({
      id: 7,
      owner: 1,
      what,
      name: "My Bike",
      last_used: new Date(),
    });
    parts.setMap([part]);
    return part;
  }

  it("shows a Notes tab for a gear", () => {
    seedPart(1);
    render(PartComponent, { id: 7 });
    expect(screen.getByRole("tab", { name: "Notes" })).toBeTruthy();
  });

  it("shows the Notes tab last for a gear", () => {
    seedPart(1);
    render(PartComponent, { id: 7 });
    const tabs = screen.getAllByRole("tab");
    expect(tabs.length).toBe(4);
    expect(tabs[0].textContent).toContain("Attached Parts");
    expect(tabs[1].textContent).toContain("Service Plans");
    expect(tabs[2].textContent).toContain("Service Logs");
    expect(tabs[3].textContent).toContain("Notes");
  });

  it("shows the Notes tab for a non-gear part", () => {
    seedPart(2);
    render(PartComponent, { id: 7 });
    const tabs = screen.getAllByRole("tab");
    expect(tabs.length).toBe(3);
    expect(tabs[0].textContent).toContain("Service Plans");
    expect(tabs[1].textContent).toContain("Service Logs");
    expect(tabs[2].textContent).toContain("Notes");
  });

  it("re-renders when the attachments collection changes", () => {
    seedPart(2);
    const { unmount } = render(PartComponent, { id: 7 });
    expect(screen.getAllByRole("tab")).toHaveLength(3);
    attachments.updateMap([
      new Attachment({
        part_id: 9,
        attached: "2023-01-01T00:00:00Z",
        gear: 7,
        hook: 1,
        detached: maxDate,
        what: 2,
        name: "Chain",
        usage: "u1",
      }),
    ]);
    flushSync();
    const tabs = screen.getAllByRole("tab");
    expect(tabs.length).toBe(4);
    expect(tabs[0].textContent).toContain("Attached Parts");
    unmount();
  });

  it("calls newNote with the part when the add button is clicked", async () => {
    seedPart(1);
    render(PartComponent, { id: 7 });
    const notesTab = screen.getByRole("tab", { name: "Notes" });
    fireEvent.click(notesTab);
    const add = await within(notesTab).findByRole("button", { name: "add" });
    fireEvent.click(add);
    await waitFor(() => {
      const newNote = (get(actions) as { newNote: unknown }).newNote;
      expect(newNote).toHaveBeenCalledWith(expect.objectContaining({ id: 7 }));
    });
  });
});
