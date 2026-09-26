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
