import aboutDeChMd from "../static/content/about.de_CH.md?raw";
import aboutDeMd from "../static/content/about.de.md?raw";
import aboutMd from "../static/content/about.md?raw";
import { describe, expect, it } from "vitest";

/// The about page is the landing content for every visitor — Strava and
/// intervals.icu alike — so its copy must not be Strava-specific. The static
/// markdown is the single source the `ContentPage` renders, so the content
/// itself is what gets asserted.
const files: [string, string][] = [
  ["about.md", aboutMd],
  ["about.de.md", aboutDeMd],
  ["about.de_CH.md", aboutDeChMd],
];

describe("about content", () => {
  it.each(files)("%s is not Strava-specific", (_file, content) => {
    expect(content).not.toContain("Strava");
  });
});
