import { fireEvent, render, screen, waitFor } from "@testing-library/svelte";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { Part } from "../lib/part";
import DeletePart from "./DeletePart.svelte";

describe("DeletePart", () => {
  let fetchMock: ReturnType<typeof vi.fn>;

  beforeEach(() => {
    fetchMock = vi.fn().mockResolvedValue({
      ok: true,
      status: 200,
      statusText: "",
      json: async () => 1,
      text: async () => "1",
    });
    vi.stubGlobal("fetch", fetchMock);
  });

  it("is closed initially", () => {
    render(DeletePart);
    expect(screen.queryByText(/Do you really want to delete/)).toBeNull();
  });

  it("opens when start() is called", async () => {
    const { component } = render(DeletePart);
    const part = new Part({
      id: 5,
      owner: 1,
      what: 1,
      name: "My Wheel",
      last_used: new Date(),
    });
    component.start(part);
    await waitFor(() => {
      expect(
        screen.getByText(/Do you really want to delete Part "My Wheel"/),
      ).toBeTruthy();
    });
  });

  it("sends DELETE request when submit button is clicked", async () => {
    const { component } = render(DeletePart);
    const part = new Part({
      id: 5,
      owner: 1,
      what: 1,
      name: "My Wheel",
      last_used: new Date(),
    });
    component.start(part);

    const submitBtn = await screen.findByRole("button", { name: "Delete" });
    fireEvent.click(submitBtn);

    await waitFor(() => {
      expect(fetchMock).toHaveBeenCalledWith(
        "/api/part/5",
        expect.objectContaining({ method: "DELETE" }),
      );
    });
  });
});
