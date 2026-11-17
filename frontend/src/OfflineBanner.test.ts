import { render, screen } from "@testing-library/svelte";
import { describe, expect, it, vi } from "vitest";
import OfflineBanner from "./OfflineBanner.svelte";
import { loadProviders } from "./lib/provider";

const BANNER_TEXT = "Running offline — no activity providers configured.";

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

describe("OfflineBanner (ticket #925)", () => {
  it("shows on a successful empty fetch (offline)", async () => {
    stubProviders([]);
    await loadProviders();
    render(OfflineBanner);
    expect(screen.getByText(BANNER_TEXT)).toBeTruthy();
  });

  it("is hidden with a non-empty provider set", async () => {
    stubProviders(["strava"]);
    await loadProviders();
    render(OfflineBanner);
    expect(screen.queryByText(BANNER_TEXT)).toBeNull();
  });

  it("is hidden on a failed fetch (a distinct error state)", async () => {
    stubProvidersFail();
    await loadProviders();
    render(OfflineBanner);
    expect(screen.queryByText(BANNER_TEXT)).toBeNull();
  });
});
