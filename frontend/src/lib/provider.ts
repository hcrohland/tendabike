/// The activity providers a user can connect with (spec #760): one per
/// user, set at signup, immutable. The values mirror the backend's
/// `users.provider` column (`'strava'` | `'icu'`).
export type Provider = "strava" | "icu";

/// The providers offered on the connect screen, in display order.
export const PROVIDERS: Provider[] = ["strava", "icu"];

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

/// The provider's onboarding backfill trigger (spec #760):
/// `/strava/onboarding/sync` or `/icu/onboarding/sync`.
export function onboardingSyncUrl(provider: Provider): string {
  return `/${provider}/onboarding/sync`;
}

/// The provider of a prefixed activity id (the `ActivityId` model,
/// spec #760): `icu:` ids are intervals.icu's, everything else is
/// Strava's.
export function providerOfActivityId(id: string): Provider {
  return id.startsWith("icu:") ? "icu" : "strava";
}
