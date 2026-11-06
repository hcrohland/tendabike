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
          headers: { get: () => "text/plain" },
          text: () => Promise.resolve(responses[path]),
        });
      }
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

  it("returns an empty string when no file matches", async () => {
    mockFetch({});
    const content = await resolveContent("nonexistent", "en");
    expect(content).toBe("");
  });

  it("ignores HTML fallback responses", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn(() =>
        Promise.resolve({
          ok: true,
          headers: { get: () => "text/html" },
          text: () => Promise.resolve("<html>...</html>"),
        }),
      ),
    );
    const content = await resolveContent("about", "en");
    expect(content).toBe("");
  });
});
