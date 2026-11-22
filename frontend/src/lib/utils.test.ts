import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  checkStatus,
  fmtDate,
  fmtNumber,
  fmtRange,
  fmtSeconds,
  get_days,
  maxDate,
  roundTime,
} from "./utils";

/// Anonymous for the 401 tests: `checkStatus` bails after the redirect
/// decision, so the user state never matters beyond "not logged in".
vi.mock("./user.svelte", () => ({
  getUser: () => undefined,
  setUser: vi.fn(),
}));

describe("checkStatus 401", () => {
  /// jsdom does not navigate: the `window.location.href = ...` assignment
  /// lands on this stub instead.
  let location: { pathname: string; search: string; href: string };

  function at(pathname: string) {
    location = {
      pathname,
      search: "",
      href: `http://localhost${pathname}`,
    };
    Object.defineProperty(window, "location", {
      value: location,
      writable: true,
    });
  }

  const unauthorized = {
    ok: false,
    status: 401,
    text: () => Promise.resolve("unauthorized"),
  } as Response;

  beforeEach(() => {
    at("/");
  });

  it("does not bounce a content page: /about", async () => {
    at("/about");
    await checkStatus(unauthorized);
    expect(location.href).toBe("http://localhost/about");
  });

  it("does not bounce a guide topic: /about/[topic]", async () => {
    at("/about/onboarding");
    await checkStatus(unauthorized);
    expect(location.href).toBe("http://localhost/about/onboarding");
  });

  it("does not bounce a content page: /data-protection", async () => {
    at("/data-protection");
    await checkStatus(unauthorized);
    expect(location.href).toBe("http://localhost/data-protection");
  });

  it("bounces a protected page to /about", async () => {
    at("/cat");
    await checkStatus(unauthorized);
    expect(location.href).toBe("/about?path=%2Fcat");
  });
});

describe("roundTime", () => {
  it("floors minutes to the nearest 15 and zeroes seconds/ms", () => {
    const res = roundTime(new Date(2023, 8, 15, 10, 23, 45, 999));
    expect(res.getHours()).toBe(10);
    expect(res.getMinutes()).toBe(15);
    expect(res.getSeconds()).toBe(0);
    expect(res.getMilliseconds()).toBe(0);
  });

  it("rounds 29 minutes down to 15", () => {
    expect(roundTime(new Date(2023, 8, 15, 10, 29, 1, 0)).getMinutes()).toBe(
      15,
    );
  });

  it("rounds 44 minutes down to 30", () => {
    expect(roundTime(new Date(2023, 8, 15, 10, 44, 59, 0)).getMinutes()).toBe(
      30,
    );
  });

  it("keeps 45 minutes as-is", () => {
    expect(roundTime(new Date(2023, 8, 15, 10, 45, 10, 0)).getMinutes()).toBe(
      45,
    );
  });
});

describe("get_days", () => {
  const start = new Date("2023-01-01T00:00:00Z");

  it("computes whole days between two dates", () => {
    expect(get_days(start, new Date("2023-01-11T00:00:00Z"))).toBe(10);
  });

  it("truncates partial days", () => {
    expect(get_days(start, new Date("2023-01-01T12:00:00Z"))).toBe(0);
  });

  it("is negative when end is before start", () => {
    expect(get_days(new Date("2023-01-10T00:00:00Z"), start)).toBe(-9);
  });
});

describe("fmtDate", () => {
  it("returns 'never' for undefined", () => {
    expect(fmtDate(undefined)).toBe("never");
  });

  it("formats a date using the navigator language", () => {
    const d = new Date(2023, 8, 15);
    expect(fmtDate(d)).toBe(d.toLocaleDateString(navigator.language));
    expect(fmtDate(d)).not.toBe("never");
  });
});

describe("fmtRange", () => {
  it("returns a single date when end is undefined", () => {
    const start = new Date(2023, 8, 15);
    expect(fmtRange(start, undefined)).toBe(fmtDate(start));
  });

  it("returns a single date when end is maxDate", () => {
    const start = new Date(2023, 8, 15);
    expect(fmtRange(start, maxDate)).toBe(fmtDate(start));
  });

  it("returns 'start - end' when end is within range", () => {
    const start = new Date(2023, 8, 1);
    const end = new Date(2023, 8, 15);
    expect(fmtRange(start, end)).toBe(fmtDate(start) + " - " + fmtDate(end));
  });
});

describe("fmtSeconds", () => {
  it("defaults to 0:00", () => {
    expect(fmtSeconds()).toBe("0:00");
  });

  it("formats minutes and zero-pads single digits", () => {
    expect(fmtSeconds(90)).toBe("0:01");
  });

  it("includes hours", () => {
    expect(fmtSeconds(3661)).toBe("1:01");
  });

  it("renders zero minutes as 00", () => {
    expect(fmtSeconds(7200)).toBe("2:00");
  });

  it("prefixes a minus for negative values", () => {
    expect(fmtSeconds(-90)).toBe("-0:01");
  });
});

describe("fmtNumber", () => {
  it("treats undefined as 0", () => {
    expect(fmtNumber(undefined)).toBe((0).toLocaleString(navigator.language));
  });

  it("keeps 0 as 0", () => {
    expect(fmtNumber(0)).toBe((0).toLocaleString(navigator.language));
  });

  it("formats a number with grouping", () => {
    expect(fmtNumber(1234567)).toBe(
      (1234567).toLocaleString(navigator.language),
    );
  });
});
