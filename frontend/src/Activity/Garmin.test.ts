import { fireEvent, render, screen, waitFor } from "@testing-library/svelte";
import { beforeEach, describe, expect, it, vi } from "vitest";
import Garmin from "./Garmin.svelte";

/// The 200 response of `POST /api/activ/descend`: the match report
/// (`{good, bad}`), the state of the matched activities riding the stream
/// frame (the spec §6.2 deviation recorded on issue #446). The body is
/// read lazily, so tests can swap it between renders.
function json200(getBody: () => object) {
  return {
    ok: true,
    status: 200,
    statusText: "OK",
    json: async () => getBody(),
  } as unknown as Response;
}

function csvFile() {
  return new File(
    [
      "Date,Title,Total Descent",
      "2023-05-18 22:13:20,Morning Ride,900",
      "2023-06-01 08:00:00,Phantom Ride,500",
    ],
    "garmin.csv",
    { type: "text/csv" },
  );
}

async function uploadAndSynchronize() {
  const input = document.querySelector('input[type="file"]')!;
  // Writable on purpose: Svelte's `bind:files` writes the state back to the
  // element, and jsdom's `files` is getter-only by default.
  Object.defineProperty(input, "files", {
    value: [csvFile()],
    writable: true,
    configurable: true,
  });
  await fireEvent.change(input);
  fireEvent.click(screen.getByText("Synchronize"));
}

describe("Garmin", () => {
  // The 200 body of the mocked upload (vi.unstubAllGlobals() runs after
  // every test — test/setup.ts — so the stub is (re)established in
  // beforeEach).
  let report: { good: string[]; bad: string[] } = {
    good: ["Morning Ride at 2023-05-18 22:13:20"],
    bad: ["Phantom Ride at 2023-06-01 08:00:00"],
  };

  beforeEach(() => {
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(json200(() => report)));
  });

  it("renders the match report of the upload", async () => {
    render(Garmin, { open: true });
    await uploadAndSynchronize();
    // The upload is an async chain (File.text() → fetch → json), so the
    // report appears in a later microtask — wait for it.
    await waitFor(() =>
      expect(document.body.textContent).toContain("Synchronized 1 activities."),
    );
    // The skipped row is listed…
    expect(
      await screen.findByText("Phantom Ride at 2023-06-01 08:00:00"),
    ).toBeTruthy();
    // …and the count line is shown.
    expect(document.body.textContent).toContain(
      "Could not match the following 1 activities:",
    );
  });

  it("renders only the success line when every row matched", async () => {
    report = { good: ["Morning Ride at 2023-05-18 22:13:20"], bad: [] };
    render(Garmin, { open: true });
    await uploadAndSynchronize();
    await waitFor(() =>
      expect(document.body.textContent).toContain("Synchronized 1 activities."),
    );
    expect(document.body.textContent).not.toContain("Could not match");
  });
});
