import { handleError, myfetch } from "./utils";

/// The activity providers a user can connect with (spec #760): one per
/// user, set at signup, immutable. The values are the wire encoding the
/// backend's `GET /api/user` response carries in `user.provider`
/// (`'strava'` | `'icu'`).
export type Provider = "strava" | "icu";

/// The per-provider branding: the display name (a brand name, not
/// translated) and the accent color the provider's chrome is painted in.
export interface ProviderMeta {
  name: string;
  accent: string;
}

export const providerMeta: Record<Provider, ProviderMeta> = {
  strava: { name: "Strava", accent: "#fc4c02" },
  icu: { name: "intervals.icu", accent: "#18181b" },
};

/// The provider's OAuth entry point: `/strava/login` or `/icu/login`.
export function loginUrl(provider: Provider): string {
  return `/${provider}/login`;
}

/// The onboarding gear import (spec #829, the first step): the
/// provider-neutral `/api/onboarding/gear` (the user's provider is
/// derived from their identity in-transaction, spec #977).
export function onboardingGearUrl(): string {
  return "/api/onboarding/gear";
}

/// The onboarding activity sync (spec #829, the second step): the
/// provider-neutral `/api/onboarding/sync` (the user's provider is
/// derived from their identity in-transaction, spec #977).
export function onboardingSyncUrl(): string {
  return "/api/onboarding/sync";
}

/// The onboarding postponement (spec #974): the provider-neutral
/// `/api/onboarding/postpone` (the user's provider is derived from
/// their identity in-transaction, spec #977).
export function onboardingPostponeUrl(): string {
  return "/api/onboarding/postpone";
}

/// The provider of a prefixed activity id (the `ActivityId` model,
/// spec #760): `icu:` ids are intervals.icu's, everything else is
/// Strava's.
export function providerOfActivityId(id: string): Provider {
  return id.startsWith("icu:") ? "icu" : "strava";
}

/// The provider's gear page URL for a part source (spec #760).
/// Strava sources are the raw Strava gear id (`b…` / `g…`); icu
/// sources are prefixed (`icu:…`). Returns `undefined` for an
/// unrecognised source.
export function gearSourceUrl(source: string): string | undefined {
  if (source.startsWith("icu:")) {
    return `https://intervals.icu/gear/${source.slice("icu:".length)}`;
  }
  if (source.startsWith("b")) {
    return `https://www.strava.com/bikes/${source.slice(1)}`;
  }
  if (source.startsWith("g")) {
    return "https://www.strava.com/settings/gear";
  }
  return undefined;
}

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
