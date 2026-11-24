import { fireEvent, render, screen, waitFor } from "@testing-library/svelte";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { Part } from "../lib/part";
import { ServicePlan } from "../lib/serviceplan";
import SnoozePlan, { snoozePlan } from "./SnoozePlan.svelte";

describe("SnoozePlan", () => {
  let fetchMock: ReturnType<typeof vi.fn>;
  const plan = new ServicePlan({
    id: "P1",
    part: 5,
    what: 10,
    name: "Wheel Service",
  });
  const part = new Part({
    id: 5,
    owner: 1,
    what: 10,
    name: "Wheel",
    last_used: new Date(),
  });

  beforeEach(() => {
    fetchMock = vi.fn().mockResolvedValue({
      ok: true,
      status: 204,
      statusText: "",
      text: async () => "",
    });
    vi.stubGlobal("fetch", fetchMock);
  });

  it("is closed initially", () => {
    render(SnoozePlan);
    expect(screen.queryByText(/Snooze/)).toBeNull();
  });

  it("opens when snoozePlan() is called", async () => {
    render(SnoozePlan);
    snoozePlan(plan, part);
    await waitFor(() => {
      expect(screen.getByText(/Snooze plan for "Wheel"/)).toBeTruthy();
    });
  });

  it("sends the snooze with the chosen fraction when submit is clicked", async () => {
    render(SnoozePlan);
    snoozePlan(plan, part);
    const input = (await screen.findByPlaceholderText(
      "Fraction",
    )) as HTMLInputElement;
    fireEvent.input(input, { target: { value: "0.5" } });

    const submitBtn = await screen.findByRole("button", { name: "Snooze" });
    fireEvent.click(submitBtn);

    await waitFor(() => {
      expect(fetchMock).toHaveBeenCalledWith(
        "/api/plan/P1/snooze",
        expect.objectContaining({
          method: "POST",
          body: JSON.stringify({ part: 5, fraction: 0.5 }),
        }),
      );
    });
  });
});
