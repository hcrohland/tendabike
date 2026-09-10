import { describe, expect, it } from "vitest";
import { getLocale } from "../../paraglide/runtime";

describe("paraglide locale", () => {
  it("resolves to en in the test environment", () => {
    expect(getLocale()).toBe("en");
  });
});
