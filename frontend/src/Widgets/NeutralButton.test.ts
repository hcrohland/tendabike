import { render, screen } from "@testing-library/svelte";
import { describe, expect, it } from "vitest";
import NeutralButtonHost from "./NeutralButtonHost.svelte";

// The neutral button is a plain element carrying the theme's single uniform
// fill role class. The component library's button is not used: its color
// classes come from a hard-coded internal table that a theme class cannot
// override, so a theme fill class only works on a plain element. We assert
// the rendered role class, never the mechanism.

describe("NeutralButton", () => {
  it("renders a plain button with the theme's fill class", () => {
    render(NeutralButtonHost, { mode: "button" });
    const btn = screen.getByRole("button", { name: "label" });
    expect(btn.classList.contains("fill")).toBe(true);
  });

  it("renders an anchor with the fill class when href is set", () => {
    render(NeutralButtonHost, { mode: "link" });
    const link = screen.getByRole("link", { name: "label" });
    expect((link as HTMLAnchorElement).href).toContain("/somewhere");
    expect(link.classList.contains("fill")).toBe(true);
  });

  it("renders a submit button when type is submit", () => {
    render(NeutralButtonHost, { mode: "submit" });
    const btn = screen.getByRole("button", { name: "label" });
    expect((btn as HTMLButtonElement).type).toBe("submit");
  });
});
