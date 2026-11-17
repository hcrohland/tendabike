import { fireEvent, render, screen } from "@testing-library/svelte";
import { beforeEach, describe, expect, it, vi } from "vitest";
import ConnectProviders from "./ConnectProviders.svelte";

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

  beforeEach(() => {
    location = { href: "http://localhost/about" };
    Object.defineProperty(window, "location", {
      value: location,
      writable: true,
    });
    stubProviders(["strava", "icu"]);
  });

  it("renders one hero card per enabled provider", async () => {
    render(ConnectProviders);
    expect(await screen.findByText("Strava")).toBeTruthy();
    expect(screen.getByText("intervals.icu")).toBeTruthy();
  });

  it("offers only the enabled providers", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn((url: string) => {
        if (url === "/api/providers") {
          return Promise.resolve({
            ok: true,
            status: 200,
            json: () => Promise.resolve(["strava"]),
          });
        }
        return Promise.reject(new Error(`unexpected fetch: ${url}`));
      }),
    );
    render(ConnectProviders);
    expect(await screen.findByText("Strava")).toBeTruthy();
    // The negative assertion must wait for the fetch to settle: the module
    // state persists across tests, so a stale enabled list renders the icu
    // card until the fresh fetch replaces it.
    await vi.waitFor(() =>
      expect(screen.queryByText("intervals.icu")).toBeNull(),
    );
  });

  it("shows the failure page when the providers fetch fails", async () => {
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
    render(ConnectProviders);
    expect(
      await screen.findByText(
        "This instance has no activity providers configured",
      ),
    ).toBeTruthy();
    expect(screen.queryByText("Strava")).toBeNull();
    expect(screen.queryByText("intervals.icu")).toBeNull();
  });

  it("shows the failure page when no provider is enabled", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn((url: string) => {
        if (url === "/api/providers") {
          return Promise.resolve({
            ok: true,
            status: 200,
            json: () => Promise.resolve([]),
          });
        }
        return Promise.reject(new Error(`unexpected fetch: ${url}`));
      }),
    );
    render(ConnectProviders);
    expect(
      await screen.findByText(
        "This instance has no activity providers configured",
      ),
    ).toBeTruthy();
    expect(screen.queryByText("Strava")).toBeNull();
    expect(screen.queryByText("intervals.icu")).toBeNull();
    expect(screen.queryByRole("button", { name: /connect/i })).toBeNull();
  });

  it("clicking the Strava card starts the Strava OAuth flow", async () => {
    render(ConnectProviders);
    fireEvent.click(
      await screen.findByRole("button", { name: "Connect Strava" }),
    );
    expect(location.href).toBe("/strava/login");
  });

  it("clicking the intervals.icu card starts the icu OAuth flow", async () => {
    render(ConnectProviders);
    fireEvent.click(
      await screen.findByRole("button", { name: "Connect intervals.icu" }),
    );
    expect(location.href).toBe("/icu/login");
  });

  it("clicking the card body (not only the button) also starts the flow", async () => {
    render(ConnectProviders);
    fireEvent.click(await screen.findByText("Rides you log in Strava"));
    expect(location.href).toBe("/strava/login");
  });
});
