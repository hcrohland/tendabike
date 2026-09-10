export function resp(body: any, status = 200, ok = true, statusText = "") {
  return {
    ok,
    status,
    statusText,
    json: async () => body,
    text: async () => (typeof body === "string" ? body : JSON.stringify(body)),
  } as unknown as Response;
}
