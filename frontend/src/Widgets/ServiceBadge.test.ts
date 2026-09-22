import { render, screen } from "@testing-library/svelte";
import { describe, expect, it } from "vitest";
import { Due } from "../lib/serviceplan";
import ServiceBadge from "./ServiceBadge.svelte";

// The badge renders only from the plan module's per-limit NextDue value:
// verdict → colour, remaining/limit → used percentage (issue #345).
// The severity thresholds are unit-tested in lib/serviceplan-doors.test.ts.

describe("ServiceBadge", () => {
  it("renders nothing when due is undefined", () => {
    render(ServiceBadge);
    expect(screen.queryByText(/%$/)).toBeNull();
  });

  it("renders a red badge for an 'alert' verdict", () => {
    render(ServiceBadge, { due: new Due(-10, 100, "alert") });
    const badge = screen.getByText(/%/);
    expect(badge.className).toMatch(/red/);
  });

  it("renders a yellow badge for a 'warn' verdict", () => {
    render(ServiceBadge, { due: new Due(3, 100, "warn") });
    const badge = screen.getByText(/%/);
    expect(badge.className).toMatch(/yellow/);
  });

  it("renders a green badge for an 'ok' verdict", () => {
    render(ServiceBadge, { due: new Due(50, 100, "ok") });
    const badge = screen.getByText("50%");
    expect(badge.className).toMatch(/green/);
  });

  it("displays the used percentage", () => {
    render(ServiceBadge, { due: new Due(25, 100, "ok") });
    expect(screen.getByText("75%")).toBeTruthy();
  });

  it("renders at the given pos", () => {
    const { container } = render(ServiceBadge, {
      due: new Due(50, 100, "ok"),
      pos: "absolute top-0 left-0",
    });
    const el = container.querySelector('[data-testid="badge-pos"]');
    expect(el?.className).toContain("absolute top-0 left-0");
  });
});
