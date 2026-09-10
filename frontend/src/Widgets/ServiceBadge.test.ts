import { render, screen } from "@testing-library/svelte";
import { describe, expect, it } from "vitest";
import ServiceBadge from "./ServiceBadge.svelte";

describe("ServiceBadge", () => {
  it("renders nothing when service is undefined", () => {
    render(ServiceBadge);
    expect(screen.queryByText(/%$/)).toBeNull();
  });

  it("renders a green badge when due is above 5% of plan", () => {
    render(ServiceBadge, { service: { due: 50, plan: 100 } });
    const badge = screen.getByText("50%");
    expect(badge.className).toMatch(/green/);
  });

  it("renders a red badge when due is negative", () => {
    render(ServiceBadge, { service: { due: -10, plan: 100 } });
    const badge = screen.getByText(/%/);
    expect(badge.className).toMatch(/red/);
  });

  it("renders a yellow badge when due is below 5% of plan but >= 0", () => {
    render(ServiceBadge, { service: { due: 3, plan: 100 } });
    const badge = screen.getByText(/%/);
    expect(badge.className).toMatch(/yellow/);
  });

  it("displays the correct percentage", () => {
    render(ServiceBadge, { service: { due: 25, plan: 100 } });
    expect(screen.getByText("75%")).toBeTruthy();
  });

  it("renders at the given pos", () => {
    const { container } = render(ServiceBadge, {
      service: { due: 50, plan: 100 },
      pos: "absolute top-0 left-0",
    });
    const el = container.querySelector('[data-testid="badge-pos"]');
    expect(el?.className).toContain("absolute top-0 left-0");
  });
});
