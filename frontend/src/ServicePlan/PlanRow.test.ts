import { fireEvent, render, screen } from "@testing-library/svelte";
import { flushSync } from "svelte";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { attachments } from "../lib/attachment";
import { Part, parts } from "../lib/part";
import { plans, ServicePlan } from "../lib/serviceplan";
import { services } from "../lib/service";
import { usages } from "../lib/usage";
import { usage } from "../test/helpers";
import PlanRow from "./PlanRow.svelte";
import { snoozePlan } from "./SnoozePlan.svelte";

vi.mock("./SnoozePlan.svelte", () => ({ snoozePlan: vi.fn() }));

describe("PlanRow", () => {
  const part = new Part({
    id: 5,
    owner: 1,
    what: 10,
    name: "Wheel",
    purchase: "2023-01-01T00:00:00Z",
    last_used: "2024-01-01T00:00:00Z",
    usage: "u1",
  });

  const plan = (overrides: Partial<any> = {}) =>
    new ServicePlan({
      id: "P1",
      part: 5,
      what: 10,
      hook: null,
      name: "Wheel Service",
      km: "100",
      ...overrides,
    });

  /// The Popper's trigger debounce (200ms, rAF) and the dropdown's fade
  /// transition (100ms, Web Animations API) each need timer advances, and
  /// Svelte schedules the animation on a microtask.
  async function settle() {
    for (let i = 0; i < 3; i++) {
      await vi.advanceTimersByTimeAsync(300);
      flushSync();
    }
  }

  async function openMenu(container: HTMLElement) {
    const trigger = container.querySelector("svg.cursor-pointer");
    expect(trigger).toBeTruthy();
    fireEvent.mouseDown(trigger!);
    await settle();
  }

  function renderRow(pln: ServicePlan, distance: number) {
    parts.setMap([part]);
    attachments.setMap([]);
    services.setMap([]);
    usages.setMap([usage("u1", { distance })]);
    plans.setMap([pln]);
    // PlanBlock passes the plan's gear to every row.
    return render(PlanRow, { plan: pln, gear: part });
  }

  beforeEach(() => {
    vi.clearAllMocks();
    vi.useFakeTimers({
      toFake: [
        "setTimeout",
        "clearTimeout",
        "setInterval",
        "clearInterval",
        "Date",
        "requestAnimationFrame",
        "cancelAnimationFrame",
        "performance",
      ],
    });
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("renders the row on the primary surface (the contrast to the enclosing block card)", () => {
    const { container } = renderRow(plan(), 50000);
    const row = container.querySelector(".rounded-lg");
    expect(row?.classList.contains("surface-primary")).toBe(true);
    expect(row?.classList.contains("surface-secondary")).toBe(false);
  });

  it("does not offer the snooze action when the (part, plan) is ok", async () => {
    const { container } = renderRow(plan(), 50000);
    await openMenu(container);
    expect(screen.getByText("New Service for plan")).toBeTruthy();
    expect(screen.queryByText("Snooze plan")).toBeNull();
  });

  it("offers the snooze action when the (part, plan) is in the warn band", async () => {
    const { container } = renderRow(plan(), 98000);
    await openMenu(container);
    expect(screen.getByText("Snooze plan")).toBeTruthy();
  });

  it("offers the snooze action when the (part, plan) is overdue", async () => {
    const { container } = renderRow(plan(), 110000);
    await openMenu(container);
    expect(screen.getByText("Snooze plan")).toBeTruthy();
  });

  it("runs the snooze operation for the (part, plan) when the action is chosen", async () => {
    const pln = plan();
    const { container } = renderRow(pln, 98000);
    await openMenu(container);
    fireEvent.click(screen.getByText("Snooze plan"));
    expect(snoozePlan).toHaveBeenCalledWith(pln, part);
  });

  it("shows a snoozed indicator with the percentage for a snoozed (part, plan)", () => {
    renderRow(plan({ snoozes: { 5: 0.5 } }), 98000);
    expect(screen.getByText("50%")).toBeTruthy();
  });

  it("shows no snoozed indicator when the (part, plan) is not snoozed", () => {
    renderRow(plan(), 98000);
    expect(screen.queryByText("50%")).toBeNull();
  });
});
