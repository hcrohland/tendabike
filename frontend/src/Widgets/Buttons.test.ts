import { fireEvent, render, screen } from "@testing-library/svelte";
import { describe, expect, it } from "vitest";
import Buttons from "./Buttons.svelte";
import ButtonsWrapper from "./ButtonsWrapper.svelte";

describe("Buttons", () => {
  it("renders a single close button when no label is provided", () => {
    render(Buttons, { open: true });
    const closeBtn = screen.getByRole("button", { name: /close/i });
    expect(closeBtn).toBeTruthy();
  });

  it("renders cancel and submit buttons when label is provided", () => {
    render(Buttons, { open: true, label: "Delete Part" });
    const cancelBtn = screen.getByRole("button", { name: /cancel/i });
    const submitBtn = screen.getByRole("button", { name: /delete part/i });
    expect(cancelBtn).toBeTruthy();
    expect(submitBtn).toBeTruthy();
  });

  it("close button renders when open is false", () => {
    render(Buttons, { open: false });
    const closeBtn = screen.getByRole("button", { name: /close/i });
    expect(closeBtn).toBeTruthy();
  });

  it("clicking close sets open to false", async () => {
    render(ButtonsWrapper);
    const closeBtn = screen.getByRole("button", { name: /close/i });
    fireEvent.click(closeBtn);
    await screen.findByText("open: false");
  });

  it("clicking cancel sets open to false", async () => {
    render(ButtonsWrapper, { label: "Delete" });
    const cancelBtn = screen.getByRole("button", { name: /cancel/i });
    fireEvent.click(cancelBtn);
    await screen.findByText("open: false");
  });
});
