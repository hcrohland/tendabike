import { describe, expect, it } from "vitest";
import { topics } from "./topics";

/// Static imports double as the content source: a renamed slug fails the
/// build here before it can break a sidebar link.
import onboarding from "../../../../static/content/about/onboarding.md?raw";
import bikesGear from "../../../../static/content/about/bikes-gear.md?raw";
import partsAssemblies from "../../../../static/content/about/parts-assemblies.md?raw";
import servicePlans from "../../../../static/content/about/service-plans.md?raw";
import activities from "../../../../static/content/about/activities.md?raw";
import shops from "../../../../static/content/about/shops.md?raw";
import statistics from "../../../../static/content/about/statistics.md?raw";

const contents: Record<string, string> = {
  onboarding,
  "bikes-gear": bikesGear,
  "parts-assemblies": partsAssemblies,
  "service-plans": servicePlans,
  activities,
  shops,
  statistics,
};

/// Lazy glob: keys only, nothing is imported (an eager glob would try to
/// parse the markdown as JS).
const fileSlugs = Object.keys(
  import.meta.glob("../../../../static/content/about/*.md"),
).map((path) => path.replace(/^.*\//, "").replace(/\.md$/, ""));

describe("guide topics", () => {
  it("every topic has a non-empty content file", () => {
    for (const topic of topics) {
      expect(
        contents[topic.slug],
        `missing content for topic ${topic.slug}`,
      ).toBeDefined();
      expect(contents[topic.slug].length).toBeGreaterThan(0);
    }
  });

  it("the sidebar lists exactly the content files that exist", () => {
    expect([...fileSlugs].sort()).toEqual(topics.map((t) => t.slug).sort());
  });
});
