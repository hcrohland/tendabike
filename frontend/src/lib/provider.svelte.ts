import { handleError, myfetch } from "./utils";
import type { Provider } from "./provider";

/// The enabled providers (issue #908): the runtime subset the server was
/// configured with, as wire values in the backend's `Provider` enum order.
/// `undefined` until [`loadProviders`] lands — the connect screen shows
/// nothing (no cards, no failure page) while the fetch is in flight.
let enabled = $state<Provider[] | undefined>(undefined);

export function getEnabledProviders(): Provider[] | undefined {
  return enabled;
}

/**
 * Load the enabled providers from `GET /api/providers` (issue #908). The
 * endpoint is unauthenticated (the connect screen is the anonymous landing
 * page) and carries no user data — it is the instance's provider
 * configuration. A failed fetch surfaces through the error notification and
 * leaves the state `undefined`.
 */
export async function loadProviders(): Promise<void> {
  const data = await myfetch("/api/providers").catch(handleError);
  if (Array.isArray(data)) {
    enabled = data as Provider[];
  }
}
