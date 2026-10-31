import { myfetch, handleError } from "./utils";

/// Read a single notification (lifecycle-aware: persistent → mark seen,
/// transient → delete). The change rides the stream frame.
export async function read(id: string) {
  await myfetch("/api/notification/" + id + "/read", "POST").catch(handleError);
}

/// Read all of the user's notifications (lifecycle-aware for each).
export async function readAll() {
  await myfetch("/api/notification/read-all", "POST").catch(handleError);
}
