import { Usage } from "../lib/usage";

export function resp(body: any, status = 200, ok = true, statusText = "") {
  return {
    ok,
    status,
    statusText,
    json: async () => body,
    text: async () => (typeof body === "string" ? body : JSON.stringify(body)),
  } as unknown as Response;
}

export function usage(id: string, o: Partial<any> = {}): Usage {
  return new Usage({
    id,
    count: 0,
    climb: 0,
    descend: 0,
    distance: 0,
    time: 0,
    duration: 0,
    energy: 0,
    ...o,
  });
}

/// The all-empty Summary fixture: every one of the nine collections empty,
/// accepting per-field overrides.
export function summary(overrides: Partial<any> = {}): any {
  return {
    parts: [],
    part_notes: [],
    attachments: [],
    activities: [],
    usages: [],
    services: [],
    plans: [],
    shops: [],
    users: [],
    ...overrides,
  };
}

/// The suite's content-rich Summary: one entry per populated collection.
export function summaryContent(): any {
  return summary({
    parts: [
      {
        id: 1,
        owner: 1,
        what: 10,
        name: "P1",
        vendor: "",
        model: "",
        purchase: "2023-01-01T00:00:00Z",
        last_used: "2024-01-01T00:00:00Z",
        disposed_at: null,
        usage: "u1",
        shop: null,
      },
    ],
    part_notes: [],
    attachments: [],
    activities: [
      {
        id: 100,
        user_id: 1,
        what: 301,
        name: "Ride",
        start: "2024-05-01T08:00:00Z",
        gear: null,
        climb: 0,
        descend: 0,
        distance: 1000,
        time: 3600,
        duration: 3600,
        energy: 100,
        device_name: "",
      },
    ],
    usages: [
      {
        id: "u1",
        count: 1,
        climb: 0,
        descend: 0,
        distance: 1000,
        time: 3600,
        duration: 3600,
        energy: 100,
      },
    ],
    services: [
      {
        id: "S1",
        part_id: 1,
        time: "2023-01-01T00:00:00Z",
        redone: "2023-01-01T00:00:00Z",
        name: "Svc",
        notes: "",
        usage: "u1",
        successor: null,
        plans: [],
      },
    ],
    plans: [
      {
        id: "PL1",
        part: 1,
        what: 10,
        hook: null,
        name: "Plan",
        days: null,
        hours: null,
        km: "100",
        climb: null,
        descend: null,
        rides: null,
        kJ: null,
      },
    ],
    shops: [
      {
        id: 10,
        owner: 1,
        name: "Shop",
        description: "",
        auto_approve: true,
        created_at: "2023-01-01T00:00:00Z",
      },
    ],
    users: [
      { id: 1, firstname: "Max", name: "Max Mustermann", avatar: undefined },
    ],
  });
}
