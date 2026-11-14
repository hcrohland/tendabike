import { render } from "@testing-library/svelte";
import { describe, expect, it } from "vitest";
import ProviderMark from "./ProviderMark.svelte";

describe("ProviderMark", () => {
  /// The mark is a leaf box with no vertical alignment of its own. Centering
  /// it next to text is the container's job (an `items-center` flex row): a
  /// mark carrying `align-middle` hangs below the line, because
  /// `vertical-align` cannot center a box taller than the text's x-height.
  it("carries no vertical alignment of its own (the container centers it)", () => {
    const { container: strava } = render(ProviderMark, { provider: "strava" });
    const { container: icu } = render(ProviderMark, { provider: "icu" });
    expect(strava.querySelector("img")?.className).not.toContain(
      "align-middle",
    );
    expect(icu.querySelector("span")?.className).not.toContain("align-middle");
  });
});
