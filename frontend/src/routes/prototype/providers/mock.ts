// PROTOTYPE (throwaway) — sample data for the provider-UX variants.
// Not wired to the backend; exists only so the mockups have realistic shape.

export type Provider = "strava" | "icu";

export const providerMeta: Record<
  Provider,
  { name: string; accent: string; blurb: string }
> = {
  strava: {
    name: "Strava",
    accent: "#fc4c02",
    blurb: "Rides you log in Strava",
  },
  icu: {
    name: "intervals.icu",
    accent: "#18181b",
    blurb: "Training data from intervals.icu",
  },
};

export type MockActivity = {
  id: string; // prefixed id per the settled activity-id model: "strava:12345" / "icu:i55751783"
  name: string;
  start: Date;
  distance: number; // meters
  time: number; // seconds
  climb: number; // meters
  descend: number; // meters
  device_name: string;
};

export const mockActivities: MockActivity[] = [
  {
    id: "strava:12345",
    name: "Morning spin",
    start: new Date("2026-11-01T07:30:00"),
    distance: 42300,
    time: 5400,
    climb: 320,
    descend: 310,
    device_name: "Edge 1040",
  },
  {
    id: "icu:i55751783",
    name: "8 × 4min intervals",
    start: new Date("2026-11-03T18:00:00"),
    distance: 38100,
    time: 6000,
    climb: 180,
    descend: 180,
    device_name: "TrainerRoad",
  },
  {
    id: "strava:12346",
    name: "Recovery loop",
    start: new Date("2026-11-04T08:15:00"),
    distance: 21500,
    time: 3600,
    climb: 90,
    descend: 95,
    device_name: "Edge 1040",
  },
];

export type MockGear = {
  id: number;
  name: string;
  vendor: string;
  model: string;
  provider: Provider;
};

export const mockGear: MockGear[] = [
  {
    id: 1,
    name: "Trek Emonda",
    vendor: "Trek",
    model: "Emonda",
    provider: "strava",
  },
  {
    id: 2,
    name: "Canyon Speedmax",
    vendor: "Canyon",
    model: "Speedmax",
    provider: "icu",
  },
];

export function providerOfActivityId(id: string): Provider {
  return id.startsWith("icu:") ? "icu" : "strava";
}
