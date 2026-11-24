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

  it("labels the input in percent", async () => {
    render(SnoozePlan);
    snoozePlan(plan, part);
    await screen.findByPlaceholderText("Percent");
  });

  it("a preset immediately snoozes with its fraction and dismisses", async () => {
    render(SnoozePlan);
    snoozePlan(plan, part);
    await screen.findByPlaceholderText("Percent");

    fireEvent.click(await screen.findByRole("button", { name: "10%" }));

    await waitFor(() => {
      expect(fetchMock).toHaveBeenCalledWith(
        "/api/plan/P1/snooze",
        expect.objectContaining({
          method: "POST",
          body: JSON.stringify({ part: 5, fraction: 0.1 }),
        }),
      );
    });
    await waitFor(() => {
      expect(screen.queryByText(/Snooze plan for "Wheel"/)).toBeNull();
    });
  });

  it("the 5% preset snoozes with fraction 0.05", async () => {
    render(SnoozePlan);
    snoozePlan(plan, part);
    fireEvent.click(await screen.findByRole("button", { name: "5%" }));

    await waitFor(() => {
      expect(fetchMock).toHaveBeenCalledWith(
        "/api/plan/P1/snooze",
        expect.objectContaining({
          method: "POST",
          body: JSON.stringify({ part: 5, fraction: 0.05 }),
        }),
      );
    });
  });

  it("the 20% preset snoozes with fraction 0.2", async () => {
    render(SnoozePlan);
    snoozePlan(plan, part);
    fireEvent.click(await screen.findByRole("button", { name: "20%" }));

    await waitFor(() => {
      expect(fetchMock).toHaveBeenCalledWith(
        "/api/plan/P1/snooze",
        expect.objectContaining({
          method: "POST",
          body: JSON.stringify({ part: 5, fraction: 0.2 }),
        }),
      );
    });
  });

  it("sends the snooze with the typed percent converted to a fraction", async () => {
    render(SnoozePlan);
    snoozePlan(plan, part);
    const input = (await screen.findByPlaceholderText(
      "Percent",
    )) as HTMLInputElement;
    fireEvent.input(input, { target: { value: "50" } });

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
