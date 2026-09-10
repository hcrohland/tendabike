import { render, screen } from "@testing-library/svelte";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { parts, Part } from "../lib/part";
import { getTypes, Type } from "../lib/types";
import { resp } from "../test/helpers";
import SelectPart from "./SelectPart.svelte";

describe("SelectPart", () => {
  let fetchMock: ReturnType<typeof vi.fn>;

  beforeEach(async () => {
    fetchMock = vi.fn().mockImplementation((url: string) => {
      if (url.includes("/api/types/part")) {
        return Promise.resolve(
          resp([
            { id: 1, name: "Tire", main: 1, hooks: [1], order: 1 },
            { id: 2, name: "Brake", main: 2, hooks: [1], order: 2 },
          ]),
        );
      }
      if (url.includes("/api/types/activity")) {
        return Promise.resolve(resp([]));
      }
      return Promise.resolve(resp(null));
    });
    vi.stubGlobal("fetch", fetchMock);
    await getTypes();
    parts.setMap([]);
  });

  it("renders a select with placeholder", () => {
    render(SelectPart, {
      type: new Type({ id: 1, name: "Tire", main: 1, hooks: [1], order: 1 }),
      part: undefined,
    });
    const select = screen.getByRole("combobox") as HTMLSelectElement;
    expect(select.options[0].textContent).toBe("Select Bike");
  });

  it("renders options for matching non-disposed parts", () => {
    parts.setMap([
      new Part({
        id: 1,
        what: 1,
        name: "Front Tire",
        last_used: new Date("2024-01-01"),
      }),
      new Part({
        id: 2,
        what: 1,
        name: "Rear Tire",
        last_used: new Date("2024-01-02"),
      }),
      new Part({
        id: 3,
        what: 2,
        name: "Brake",
        last_used: new Date("2024-01-01"),
      }),
    ]);
    const type = new Type({
      id: 1,
      name: "Tire",
      main: 1,
      hooks: [1],
      order: 1,
    });
    render(SelectPart, { type, part: undefined });
    expect(screen.getByText("Front Tire")).toBeTruthy();
    expect(screen.getByText("Rear Tire")).toBeTruthy();
    expect(screen.queryByText("Brake")).toBeNull();
  });

  it("excludes disposed parts", () => {
    parts.setMap([
      new Part({
        id: 1,
        what: 1,
        name: "Old Tire",
        last_used: new Date("2024-01-01"),
        disposed_at: new Date("2024-06-01"),
      }),
      new Part({
        id: 2,
        what: 1,
        name: "New Tire",
        last_used: new Date("2024-01-01"),
      }),
    ]);
    const type = new Type({
      id: 1,
      name: "Tire",
      main: 1,
      hooks: [1],
      order: 1,
    });
    render(SelectPart, { type, part: undefined });
    expect(screen.queryByText("Old Tire")).toBeNull();
    expect(screen.getByText("New Tire")).toBeTruthy();
  });

  it("renders none option when none prop is true", () => {
    const type = new Type({
      id: 1,
      name: "Tire",
      main: 1,
      hooks: [1],
      order: 1,
    });
    render(SelectPart, { type, part: undefined, none: true });
    const select = screen.getByRole("combobox") as HTMLSelectElement;
    const noneOption = Array.from(select.options).find(
      (o) => o.textContent?.trim() === "-- None --",
    );
    expect(noneOption).toBeTruthy();
  });
});
