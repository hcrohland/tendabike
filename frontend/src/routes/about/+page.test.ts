import { render } from "@testing-library/svelte";
import { describe, expect, it, vi } from "vitest";
import AboutPage from "./+page.svelte";

/// The mocked `page.url.pathname`, as in Header.test.ts.
const appState = vi.hoisted(() => ({ pathname: "/about" }));

vi.mock("$app/state", () => ({
  page: {
    url: {
      get pathname() {
        return appState.pathname;
      },
    },
  },
}));
vi.mock("../../lib/user.svelte", () => ({
  getUser: () => undefined,
}));
vi.stubGlobal(
  "fetch",
  vi.fn((url: string) => {
    if (url.startsWith("/content/about")) {
      return Promise.resolve({
        ok: true,
        text: () => Promise.resolve("# TendaBike\n"),
      });
    }
    return Promise.reject(new Error(`unexpected fetch: ${url}`));
  }),
);

/// The connect cards are the anonymous landing content: they must sit above
/// the guide row at full page width, never inside it (the sidebar must not
/// cover them).
describe("about page (anonymous)", () => {
  it("renders the connect section above the guide row, outside it", async () => {
    render(AboutPage);

    const connect = document.querySelector("#connect");
    const sidebar = document
      .querySelector('a[href="/about/onboarding"]')
      ?.closest("nav");

    expect(connect).toBeTruthy();
    expect(sidebar).toBeTruthy();

    // The sidebar lives in the guide row; the connect section does not.
    expect(sidebar?.parentElement?.className).toContain("flex");
    expect(connect?.parentElement).not.toBe(sidebar?.parentElement);

    // And the connect section comes first.
    expect(
      connect!.compareDocumentPosition(sidebar!) &
        Node.DOCUMENT_POSITION_FOLLOWING,
    ).toBeTruthy();
  });
});
