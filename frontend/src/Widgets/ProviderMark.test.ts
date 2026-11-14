import { render } from "@testing-library/svelte";
import { describe, expect, it } from "vitest";
import ProviderMark from "./ProviderMark.svelte";

describe("ProviderMark", () => {
  /// The strava mark is an `<img>` and the icu mark is an `<span>`: the two
  /// box types sit on the text baseline differently (an inline image's
  /// baseline is its bottom edge; an inline-flex span's is its content's),
  /// so the icu badge rendered lower than the strava icon. Both carry
  /// `align-middle` so they center on the same line.
  it("aligns the strava and icu marks on the same baseline", () => {
    const { container: strava } = render(ProviderMark, { provider: "strava" });
    const { container: icu } = render(ProviderMark, { provider: "icu" });
    expect(strava.querySelector("img")?.className).toContain("align-middle");
    expect(icu.querySelector("span")?.className).toContain("align-middle");
  });
});
