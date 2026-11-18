import { render, screen } from "@testing-library/svelte";
import { describe, expect, it, vi } from "vitest";
import Header from "./Header.svelte";

/// The mocked `page.url.pathname`, set per test (a getter over a hoisted
/// holder, since the real `page` type is read-only).
const appState = vi.hoisted(() => ({ pathname: "" }));

vi.mock("$app/state", () => ({
  page: {
    url: {
      get pathname() {
        return appState.pathname;
      },
    },
  },
}));
vi.mock("./lib/user.svelte", () => ({
  getUser: () => undefined,
  refresh: vi.fn(),
  setUser: vi.fn(),
}));
vi.mock("./lib/stream", () => ({
  markHydrated: vi.fn(),
  startStream: vi.fn(),
  stopStream: vi.fn(),
}));
vi.mock("./lib/types.svelte", () => ({
  getCategory: () => ({ name: "Bike" }),
}));

/// The connect button (`/about#connect`) and the connect cards (the /about
/// landing content) are both shown to anonymous visitors. On /about the button
/// just points at the cards that are already there, so it must be hidden there
/// and shown elsewhere.
describe("Header connect button", () => {
  function connectButton() {
    return document.querySelector('a[href="/about#connect"]');
  }

  it("is hidden on /about (the connect cards are already there)", () => {
    appState.pathname = "/about";
    render(Header);
    expect(connectButton()).toBeNull();
  });

  it("is shown elsewhere for an anonymous visitor", () => {
    appState.pathname = "/cat";
    render(Header);
    expect(connectButton()).toBeTruthy();
    expect(screen.getByText("Connect")).toBeTruthy();
  });
});
