import { render, screen } from "@testing-library/svelte";
import { describe, expect, it } from "vitest";
import { Due } from "../lib/serviceplan";
import Chip from "./Chip.svelte";

describe("Chip", () => {
  it("renders value and label", () => {
    render(Chip, { label: "Rides", value: "5" });
    expect(screen.getByText("5")).toBeTruthy();
    expect(screen.getByText("Rides")).toBeTruthy();
  });

  it("renders as a plain div without onclick or href", () => {
    render(Chip, { label: "Label", value: "Val" });
    const el = screen.getByText("Val");
    expect(el.closest("a")).toBeNull();
    expect(el.closest("button")).toBeNull();
  });

  it("renders a button when onclick is provided", () => {
    render(Chip, { label: "L", value: "V", onclick: () => {} });
    const btn = screen.getByRole("button");
    expect(btn).toBeTruthy();
  });

  it("renders a link when href is provided", () => {
    render(Chip, { label: "L", value: "V", href: "/part/1" });
    const link = screen.getByRole("link");
    expect(link).toBeTruthy();
    expect((link as HTMLAnchorElement).href).toContain("/part/1");
  });

  it("renders a spinner when disabled and indicator set", () => {
    render(Chip, {
      label: "L",
      value: "V",
      onclick: () => {},
      disabled: true,
      indicator: "syncing",
    });
    const spinner = screen.getByRole("status");
    expect(spinner).toBeTruthy();
    expect(screen.getByText("V")).toBeTruthy();
  });

  it("renders a service badge when the due prop is provided", () => {
    render(Chip, {
      label: "L",
      value: "V",
      due: new Due(50, 100, "ok"),
    });
    const badge = screen.getByText("50%");
    expect(badge).toBeTruthy();
  });

  it("applies light background class", () => {
    render(Chip, { label: "L", value: "V", light: true });
    const el = screen.getByText("V").closest(".bg-surface-2");
    expect(el).toBeTruthy();
  });
});
