import { describe, expect, it, vi } from "vitest";
import {
  getEnabledProviders,
  getProvidersStatus,
  isOffline,
  loadProviders,
} from "./provider";

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

describe("provider state", () => {
  /// The module state starts in flight: `loading`, not offline. Must run
  /// first — the module state persists across the file's tests.
  it("starts loading: not offline, no providers", () => {
    expect(getProvidersStatus()).toBe("loading");
    expect(getEnabledProviders()).toEqual([]);
    expect(isOffline()).toBe(false);
  });

  it("a successful empty response is offline", async () => {
    stubProviders([]);
    await loadProviders();
    expect(getProvidersStatus()).toBe("ready");
    expect(getEnabledProviders()).toEqual([]);
    expect(isOffline()).toBe(true);
  });

  it("a successful non-empty response is not offline", async () => {
    stubProviders(["strava"]);
    await loadProviders();
    expect(getProvidersStatus()).toBe("ready");
    expect(getEnabledProviders()).toEqual(["strava"]);
    expect(isOffline()).toBe(false);
  });

  it("a failed fetch is an error, not offline", async () => {
    stubProvidersFail();
    await loadProviders();
    expect(getProvidersStatus()).toBe("error");
    expect(getEnabledProviders()).toEqual([]);
    expect(isOffline()).toBe(false);
  });
});
