import { fireEvent, render, screen } from "@testing-library/svelte";
import { flushSync } from "svelte";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import DateTime from "./DateTime.svelte";
import { getLocale, overwriteGetLocale } from "../../paraglide/runtime";

// The combined-popover picker shows the date + time in a single trigger
// button, formatted in the client's locale. The popover (inline Datepicker +
// time input) is closed by default, so the button label is what the user sees.

const realGetLocale = getLocale;

afterEach(() => {
  overwriteGetLocale(realGetLocale);
});

function useLocale(locale: "en" | "de") {
  overwriteGetLocale(() => locale);
  expect(getLocale()).toBe(locale);
}

// The contract: whatever date the form holds, the trigger button must show
// that date (in the client's locale) plus the time.
function expectLabel(date: Date, locale: "en" | "de") {
  const btn = screen.getByRole("button", { name: /\d{4}/ });
  const dateOnly = new Date(
    date.getFullYear(),
    date.getMonth(),
    date.getDate(),
  );
  const time = `${String(date.getHours()).padStart(2, "0")}:${String(
    date.getMinutes(),
  ).padStart(2, "0")}`;
  expect(btn.textContent?.trim()).toBe(
    `${dateOnly.toLocaleDateString(locale)} ${time}`,
  );
}

describe("DateTime (client locale)", () => {
  it("shows the given date in the German locale", () => {
    useLocale("de");
    const date = new Date(2026, 9, 8, 14, 30); // 8. Okt 2026
    render(DateTime, { props: { date } });
    expectLabel(date, "de");
  });

  it("shows a umlaut month in the German locale", () => {
    // non-ASCII month names must not break the locale display
    useLocale("de");
    const date = new Date(2026, 2, 8, 14, 30); // 8. Mär 2026
    render(DateTime, { props: { date } });
    expectLabel(date, "de");
  });

  it("keeps working in the English locale", () => {
    useLocale("en");
    const date = new Date(2026, 9, 8, 14, 30); // 8. Oct 2026
    render(DateTime, { props: { date } });
    expectLabel(date, "en");
  });

  it("shows a month whose name exists in both locales (Nov)", () => {
    useLocale("de");
    const date = new Date(2026, 10, 5, 14, 30); // 5. Nov 2026
    render(DateTime, { props: { date } });
    expectLabel(date, "de");
  });
});

describe("DateTime (time rounding)", () => {
  // The Popper toggles on a requestAnimationFrame debounce that compares
  // against performance.now(); both must be faked for the popover to open
  // deterministically (same setup as NotificationBell.test.ts).
  beforeEach(() => {
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

  async function settle() {
    for (let i = 0; i < 3; i++) {
      await vi.advanceTimersByTimeAsync(300);
      flushSync();
    }
  }

  it("snaps a picked time to the 15-minute grid", async () => {
    useLocale("en");
    const date = new Date(2026, 9, 8, 14, 30);
    const { container } = render(DateTime, { props: { date } });

    // open the popover (the Popper trigger listens for mousedown)
    const btn = screen.getByRole("button", { name: /\d{4}/ });
    fireEvent.mouseDown(btn);
    await settle();

    const timeInput = container.querySelector(
      'input[type="time"]',
    ) as HTMLInputElement;
    expect(timeInput).toBeTruthy();
    fireEvent.change(timeInput, { target: { value: "14:37" } });
    await settle();

    // 14:37 snaps down to 14:30
    expect(
      screen.getByRole("button", { name: /14:30/ }).textContent?.trim(),
    ).toBe("10/8/2026 14:30");
  });
});
