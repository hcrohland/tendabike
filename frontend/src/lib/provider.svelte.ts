import { handleError, myfetch } from "./utils";
import type { Provider } from "./provider";

/// The load status of the enabled providers: `loading` until
/// [`loadProviders`] settles, then `ready` (a successful fetch — the set
/// may be empty: the instance runs offline) or `error` (the fetch failed).
let status = $state<"loading" | "ready" | "error">("loading");

/// The enabled providers: the runtime subset the server was configured
/// with, as wire values in the backend's `Provider` enum order. Empty
/// while the fetch is in flight and after a failed fetch.
let enabled = $state<Provider[]>([]);

export function getProvidersStatus(): "loading" | "ready" | "error" {
  return status;
}

export function getEnabledProviders(): Provider[] {
  return enabled;
}

/// The offline state: a *successful* `/api/providers` fetch
/// that returned an empty enabled set. A failed fetch is a distinct error
/// state and is not offline.
export function isOffline(): boolean {
  return status === "ready" && enabled.length === 0;
}

/**
 * Load the enabled providers from `GET /api/providers`. The endpoint is
 * unauthenticated (the connect screen is the anonymous landing page) and
 * carries no user data — it is the instance's provider configuration. A
 * failed fetch surfaces through the error notification and leaves the
 * state in `error`: the connect screen renders its "couldn't load" state,
 * distinct from the offline message.
 */
export async function loadProviders(): Promise<void> {
  const data = await myfetch("/api/providers")
    .then((d) => (Array.isArray(d) ? (d as Provider[]) : []))
    .catch(handleError);
  if (data === undefined) {
    enabled = [];
    status = "error";
  } else {
    enabled = data;
    status = "ready";
  }
}
