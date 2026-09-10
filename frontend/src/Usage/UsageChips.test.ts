import { render, screen } from "@testing-library/svelte";
import { describe, expect, it } from "vitest";
import { Usage } from "../lib/usage";
import UsageChips from "./UsageChips.svelte";

describe("UsageChips", () => {
  it("renders zero values when no usage is provided", () => {
    render(UsageChips, { id: undefined, ref: undefined });
    expect(screen.getAllByText("0").length).toBeGreaterThanOrEqual(3);
  });

  it("renders usage values when usage prop is set", () => {
    const usage = new Usage({
      id: "1",
      count: 42,
      climb: 100,
      descend: 50,
      distance: 25000,
      time: 7200,
      duration: 7200,
      energy: 0,
    });
    render(UsageChips, { usage, ref: 1 });
    expect(screen.getByText("42")).toBeTruthy();
    expect(screen.getByText("25")).toBeTruthy();
    expect(screen.getByText("2:00")).toBeTruthy();
  });

  it("shows energy chip only when energy > 0", () => {
    const noEnergy = new Usage({
      id: "1",
      count: 1,
      climb: 0,
      descend: 0,
      distance: 1000,
      time: 60,
      duration: 60,
      energy: 0,
    });
    const { unmount } = render(UsageChips, { usage: noEnergy, ref: 1 });
    expect(screen.queryByText("0 kJ")).toBeNull();
    unmount();

    const withEnergy = new Usage({
      id: "1",
      count: 1,
      climb: 0,
      descend: 0,
      distance: 1000,
      time: 60,
      duration: 60,
      energy: 250,
    });
    render(UsageChips, { usage: withEnergy, ref: 1 });
    expect(screen.getByText("250")).toBeTruthy();
  });

  it("renders link to activities when ref is provided", () => {
    const usage = new Usage({
      id: "1",
      count: 1,
      climb: 0,
      descend: 0,
      distance: 1000,
      time: 60,
      duration: 60,
      energy: 0,
    });
    render(UsageChips, { usage, ref: 99 });
    const link = screen.getByRole("link");
    expect((link as HTMLAnchorElement).href).toContain("/activities/99");
  });
});
