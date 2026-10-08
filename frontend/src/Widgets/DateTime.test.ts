import { render, screen } from "@testing-library/svelte";
import { afterEach, describe, expect, it } from "vitest";
import DateTime from "./DateTime.svelte";
import { formatDate, parseDate } from "svelty-picker";
import { de, en } from "svelty-picker/i18n";
import { getLocale, overwriteGetLocale } from "../../paraglide/runtime";

// Issue #483: the picker round-trips its string value across a locale
// boundary. The SveltyPicker component formats/parses with the i18n it is
// given (default: en), while the widget's bind:value accessor uses the
// client locale (de). Month names that differ between the two ("Okt" vs
// "Oct") then fail the cross-locale parse (indexOf -1 -> month 0 ->
// wraps to December), so a German client sees a date two months off.
// (Non-ASCII month names like "Mär" are safe: the library's token split
// treats any non-ASCII character as a token character.)

const realGetLocale = getLocale;

afterEach(() => {
  overwriteGetLocale(realGetLocale);
});

function useLocale(locale: "en" | "de") {
  overwriteGetLocale(() => locale);
  expect(getLocale()).toBe(locale);
}

// The contract: whatever date the form holds, the picker's input must
// display exactly that date, in the client's locale.
function expectDisplays(date: Date, locale: "en" | "de") {
  const input = screen.getByRole("textbox") as HTMLInputElement;
  expect(input.value).toBe(
    formatDate(date, "d. M yyyy - h:ii", locale === "de" ? de : en, "standard"),
  );
  // and the displayed string parses back to the same date
  const parsed = parseDate(
    input.value,
    "d. M yyyy - h:ii",
    locale === "de" ? de : en,
    "standard",
  );
  expect(parsed.getTime()).toBe(date.getTime());
}

describe("DateTime (client locale)", () => {
  it("displays the given date in the German locale (issue #483)", () => {
    useLocale("de");
    const date = new Date(2026, 9, 8, 14, 30); // 8. Okt 2026
    render(DateTime, { props: { date } });
    expectDisplays(date, "de");
  });

  it("displays an umlaut month (Mär) in the German locale (issue #483)", () => {
    // non-ASCII month names must survive the round-trip in the client locale
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
