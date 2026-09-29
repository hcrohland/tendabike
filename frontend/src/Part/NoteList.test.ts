import { fireEvent, render, screen, waitFor } from "@testing-library/svelte";
import { flushSync } from "svelte";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { Part } from "../lib/part";
import { partNotes, PartNote } from "../lib/partnote";
import { newNote } from "./NewNote.svelte";
import { deleteNote } from "./DeleteNote.svelte";
import NoteList from "./NoteList.svelte";

vi.mock("./NewNote.svelte", () => ({ newNote: vi.fn() }));
vi.mock("./DeleteNote.svelte", () => ({ deleteNote: vi.fn() }));

describe("NoteList", () => {
  const part = new Part({
    id: 7,
    owner: 1,
    what: 1,
    name: "My Bike",
    last_used: new Date(),
  });

  const textNote = new PartNote({
    id: 101,
    part: 7,
    name: "Replace chain soon",
    created: new Date("2026-01-01"),
  });
  const fileNote = new PartNote({
    id: 102,
    part: 7,
    name: "Receipt",
    mime: "application/pdf",
    filename: "receipt.pdf",
    size: 2048,
    created: new Date("2026-01-02"),
  });

  async function openRowMenu(noteName: string) {
    vi.useFakeTimers({ toFake: ["requestAnimationFrame", "performance"] });
    try {
      const row = screen.getByText(noteName).closest(".flex")!;
      const trigger = row.querySelector("svg")!;
      fireEvent.mouseDown(trigger);
      await vi.advanceTimersByTimeAsync(300);
    } finally {
      vi.useRealTimers();
    }
  }

  beforeEach(() => {
    vi.clearAllMocks();
    partNotes.setMap([]);
  });

  it("renders nothing for a part without notes", () => {
    partNotes.setMap([
      new PartNote({
        id: 999,
        part: 8,
        name: "Some other part's note",
        created: new Date(),
      }),
    ]);
    render(NoteList, { part });
    expect(screen.queryByText("Some other part's note")).toBeNull();
  });

  it("renders the notes of the part", () => {
    partNotes.setMap([textNote, fileNote]);
    render(NoteList, { part });
    expect(screen.getByText("Replace chain soon")).toBeTruthy();
    expect(screen.getByText("Receipt")).toBeTruthy();
  });

  it("re-renders when the partNotes collection changes", () => {
    const { unmount } = render(NoteList, { part });
    expect(screen.queryByText("Replace chain soon")).toBeNull();
    partNotes.updateMap([textNote]);
    flushSync();
    expect(screen.getByText("Replace chain soon")).toBeTruthy();
    unmount();
  });

  it("links the file of a file note", () => {
    partNotes.setMap([fileNote]);
    render(NoteList, { part });
    const link = screen.getByRole("link");
    expect(link.getAttribute("href")).toBe("/api/part/notes/102/file");
  });

  it("opens the row menu with change and delete actions", async () => {
    partNotes.setMap([textNote]);
    render(NoteList, { part });
    await openRowMenu("Replace chain soon");
    const change = await screen.findByText("Change note");
    const del = await screen.findByText("Delete note");
    expect(change).toBeTruthy();
    expect(del).toBeTruthy();
  });

  it("calls newNote with the part and note when change is chosen", async () => {
    partNotes.setMap([textNote]);
    render(NoteList, { part });
    await openRowMenu("Replace chain soon");
    const change = await screen.findByText("Change note");
    fireEvent.click(change);
    await waitFor(() => {
      expect(newNote).toHaveBeenCalledWith(part, textNote);
    });
  });

  it("calls deleteNote with the note when delete is chosen", async () => {
    partNotes.setMap([textNote]);
    render(NoteList, { part });
    await openRowMenu("Replace chain soon");
    const del = await screen.findByText("Delete note");
    fireEvent.click(del);
    await waitFor(() => {
      expect(deleteNote).toHaveBeenCalledWith(textNote);
    });
  });
});
