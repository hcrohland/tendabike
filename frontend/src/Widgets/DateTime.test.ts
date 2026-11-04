import { render, screen } from "@testing-library/svelte";
import { afterEach, describe, expect, it } from "vitest";
import DateTime from "./DateTime.svelte";
import { getLocale, overwriteGetLocale } from "../../paraglide/runtime";

// Issue #483: the picker must display the date in the client's locale. The
// Datepicker formats its input with `toLocaleDateString(locale)`, so whatever
// date the form holds, the date input shows it in the client locale — a
// German client sees "8.10.2026", an English client "10/8/2026". The time
// lives in a separate time input, so the date input carries only the day.

const realGetLocale = getLocale;

afterEach(() => {
  overwriteGetLocale(realGetLocale);
});

function useLocale(locale: "en" | "de") {
  overwriteGetLocale(() => locale);
  expect(getLocale()).toBe(locale);
}

// The contract: whatever date the form holds, the date input must display
// exactly that date, in the client's locale.
function expectDisplays(date: Date, locale: "en" | "de") {
  const input = screen.getByRole("textbox") as HTMLInputElement;
  const dateOnly = new Date(
    date.getFullYear(),
    date.getMonth(),
    date.getDate(),
  );
  expect(input.value).toBe(dateOnly.toLocaleDateString(locale));
}

describe("DateTime (client locale)", () => {
  it("displays the given date in the German locale (issue #483)", () => {
    useLocale("de");
    const date = new Date(2026, 9, 8, 14, 30); // 8. Okt 2026
    render(DateTime, { props: { date } });
    expectDisplays(date, "de");
  });

  it("displays a umlaut month in the German locale (issue #483)", () => {
    // non-ASCII month names must not break the locale display
    useLocale("de");
    const date = new Date(2026, 2, 8, 14, 30); // 8. Mär 2026
    render(DateTime, { props: { date } });
    expectDisplays(date, "de");
  });

  it("keeps working in the English locale", () => {
    useLocale("en");
    const date = new Date(2026, 9, 8, 14, 30); // 8. Oct 2026
    render(DateTime, { props: { date } });
    expectDisplays(date, "en");
  });

  it("displays a month whose name exists in both locales (Nov)", () => {
    useLocale("de");
    const date = new Date(2026, 10, 5, 14, 30); // 5. Nov 2026
    render(DateTime, { props: { date } });
    expectDisplays(date, "de");
  });
});
