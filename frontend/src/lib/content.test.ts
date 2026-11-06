import { describe, it, expect, vi, beforeEach } from "vitest";
import { resolveContent } from "./content";

function mockFetch(responses: Record<string, string>) {
  vi.stubGlobal(
    "fetch",
    vi.fn((url: string) => {
      const path = url.replace(/^\/content\//, "").replace(/\.md$/, "");
      if (path in responses) {
        return Promise.resolve({
          ok: true,
          headers: { get: () => "text/markdown" },
          text: () => Promise.resolve(responses[path]),
        });
      }
      // Simulates ServeDir fallback: 200 with HTML
      return Promise.resolve({
        ok: true,
        headers: { get: () => "text/html" },
        text: () => Promise.resolve("<html>fallback</html>"),
      });
    }),
  );
}

describe("resolveContent", () => {
  beforeEach(() => {
    vi.unstubAllGlobals();
  });

  it("returns the locale-specific file when it exists", async () => {
    mockFetch({
      "about.de": "German content",
      about: "Fallback content",
    });
    const content = await resolveContent("about", "de");
    expect(content).toBe("German content");
  });

  it("falls back to the base file when no locale-specific file exists", async () => {
    mockFetch({
      "data-protection": "English fallback",
    });
    const content = await resolveContent("data-protection", "de");
    expect(content).toBe("English fallback");
  });

  it("returns the base file for the base locale when no locale-specific file exists", async () => {
    mockFetch({
      about: "English content",
    });
    const content = await resolveContent("about", "en");
    expect(content).toBe("English content");
  });

  it("returns an empty string when only HTML fallback is available", async () => {
    mockFetch({});
    const content = await resolveContent("nonexistent", "en");
    expect(content).toBe("");
  });
});
