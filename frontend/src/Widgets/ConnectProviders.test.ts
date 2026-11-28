import { fireEvent, render, screen } from "@testing-library/svelte";
import { beforeEach, describe, expect, it, vi } from "vitest";
import ConnectProviders from "./ConnectProviders.svelte";
import { loadProviders } from "../lib/provider.svelte";

const OFFLINE_TEXT =
  "We're running offline. No activity providers are configured, so importing and syncing activities is unavailable. Add a provider to get online.";
const LOAD_FAILED_TEXT = "Couldn't load providers. Please try again.";

describe("ConnectProviders", () => {
  /// jsdom does not navigate: the component's `window.location.href = ...`
  /// assignment lands on this stub instead.
  let location: { href: string };

  /// Stub the `GET /api/providers` fetch with the enabled provider wire
  /// values.
  function stubProviders(providers: string[]) {
    vi.stubGlobal(
      "fetch",
      vi.fn((url: string) => {
        if (url === "/api/providers") {
          return Promise.resolve({
            ok: true,
            status: 200,
            json: () => Promise.resolve(providers),
          });
        }
        return Promise.reject(new Error(`unexpected fetch: ${url}`));
      }),
    );
  }

  /// Stub the `GET /api/providers` fetch as failing.
  function stubProvidersFail() {
    vi.stubGlobal(
      "fetch",
      vi.fn((url: string) => {
        if (url === "/api/providers") {
          return Promise.resolve({
            ok: false,
            status: 500,
            text: () => Promise.resolve("internal server error"),
          });
        }
        return Promise.reject(new Error(`unexpected fetch: ${url}`));
      }),
    );
  }

  beforeEach(() => {
    location = { href: "http://localhost/about" };
    Object.defineProperty(window, "location", {
      value: location,
      writable: true,
    });
  });

  /// The component reads the app-wide provider state: the
  /// layout fetches it, so the test drives it the same way.
  async function load(providers: string[] | "fail") {
    if (providers === "fail") stubProvidersFail();
    else stubProviders(providers);
    await loadProviders();
  }

  it("shows nothing while the fetch is in flight", () => {
    // No loadProviders() call: the state is still `loading`. Must run
    // first — the module state persists across the file's tests.
    render(ConnectProviders);
    expect(screen.queryByText(OFFLINE_TEXT)).toBeNull();
    expect(screen.queryByText(LOAD_FAILED_TEXT)).toBeNull();
    expect(screen.queryByText("Strava")).toBeNull();
    expect(screen.queryByText("intervals.icu")).toBeNull();
  });

  it("renders one hero card per enabled provider", async () => {
    await load(["strava", "icu"]);
    render(ConnectProviders);
    expect(await screen.findByText("Strava")).toBeTruthy();
    expect(screen.getByText("intervals.icu")).toBeTruthy();
  });

  it("renders each provider card on the primary (light) surface", async () => {
    await load(["strava", "icu"]);
    render(ConnectProviders);
    const strava = await screen.findByText("Strava");
    // the card the Strava name sits in is the light (primary) surface
    const card = strava.closest(".surface-primary");
    expect(card).toBeTruthy();
    // a primary surface is the light tone, not the darker one
    expect(card?.classList.contains("surface-secondary")).toBe(false);
  });

  it("offers only the enabled providers", async () => {
    await load(["strava"]);
    render(ConnectProviders);
    expect(await screen.findByText("Strava")).toBeTruthy();
    expect(screen.queryByText("intervals.icu")).toBeNull();
  });

  it("shows the offline message when no provider is enabled", async () => {
    await load([]);
    render(ConnectProviders);
    expect(await screen.findByText(OFFLINE_TEXT)).toBeTruthy();
    expect(screen.queryByText("Strava")).toBeNull();
    expect(screen.queryByText("intervals.icu")).toBeNull();
    expect(screen.queryByRole("button", { name: /connect/i })).toBeNull();
  });

  it("shows a 'couldn't load' state on a failed fetch, not the offline message", async () => {
    await load("fail");
    render(ConnectProviders);
    expect(await screen.findByText(LOAD_FAILED_TEXT)).toBeTruthy();
    expect(screen.queryByText(OFFLINE_TEXT)).toBeNull();
    expect(screen.queryByText("Strava")).toBeNull();
    expect(screen.queryByText("intervals.icu")).toBeNull();
  });

  it("clicking the Strava card starts the Strava OAuth flow", async () => {
    await load(["strava", "icu"]);
    render(ConnectProviders);
    fireEvent.click(
      await screen.findByRole("button", { name: "Connect Strava" }),
    );
    expect(location.href).toBe("/strava/login");
  });

  it("clicking the intervals.icu card starts the icu OAuth flow", async () => {
    await load(["strava", "icu"]);
    render(ConnectProviders);
    fireEvent.click(
      await screen.findByRole("button", { name: "Connect intervals.icu" }),
    );
    expect(location.href).toBe("/icu/login");
  });

  it("clicking the card body (not only the button) also starts the flow", async () => {
    await load(["strava", "icu"]);
    render(ConnectProviders);
    fireEvent.click(await screen.findByText("Rides you log in Strava"));
    expect(location.href).toBe("/strava/login");
  });
});
