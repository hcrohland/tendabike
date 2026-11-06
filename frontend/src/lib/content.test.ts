import { describe, it, expect } from "vitest";
import { resolveContent } from "./content";

describe("resolveContent", () => {
  it("returns the locale-specific file when it exists", () => {
    const content = resolveContent("about", "de");
    expect(content).toContain("Behalte im Blick");
  });

  it("returns the locale-specific file for de_CH when it exists", () => {
    const content = resolveContent("about", "de_CH");
    expect(content).toContain("Behalt im Blick");
  });

  it("falls back to the base file when no locale-specific file exists", () => {
    // data-protection only has a base file, no locale variants
    const content = resolveContent("data-protection", "de");
    expect(content).toContain("Data Protection");
  });

  it("returns the base file for the base locale when no locale-specific file exists", () => {
    const content = resolveContent("about", "en");
    // about.en.md doesn't exist, so it falls back to about.md
    expect(content).toContain("Know how long your parts last");
  });

  it("returns an empty string when no file matches", () => {
    const content = resolveContent("nonexistent", "en");
    expect(content).toBe("");
  });
});
