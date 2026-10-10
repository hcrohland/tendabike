import { handleError, myfetch } from "./store";
import { refresh, updateSummary, type Summary } from "./user";
import { getShop } from "./shop";

/// The native EventSource is not available in jsdom, so the suite injects a
/// stand-in factory; the app uses the native one. The seam is the surface
/// the stream uses, nothing more.
export type EventSourceLike = Pick<
  EventSource,
  "onopen" | "onmessage" | "onerror" | "close"
>;
export type EventSourceFactory = (url: string) => EventSourceLike;

/// The session-scoped SSE endpoint: each `data:` frame carries the full
/// `Summary` JSON of one completed executor message, no envelope.
const streamUrl = "/api/user/stream";

/// Consecutive failed opens before the stream probes liveness with a real
/// fetch: `EventSource` never exposes the response status, so the probe is
/// the only way to let the 401 → login redirect fire.
const maxFailedOpens = 5;

/// The catch-up snapshot's bounded retry (spec §7: "retried on failure"):
/// the first attempt plus two retries, spaced at the native `EventSource`
/// reconnect cadence (~3s). Upserts keep flowing on the live stream in the
/// meantime; deletions wait for the snapshot.
const snapshotRetries = 2;
const snapshotRetryDelay = 3000;

let active: UserStream | undefined;

/// Open the user stream and keep it open. Returns the promise of the first
/// catch-up snapshot; every snapshot (initial and reconnect) is also passed
/// to `onSnapshot`, so the caller can keep the avatar spinner in step.
export function startStream(
  onSnapshot: (p: Promise<void>) => void,
  factory: EventSourceFactory = (url) => new EventSource(url),
): Promise<void> {
  stopStream();
  active = new UserStream(factory);
  return active.start(onSnapshot);
}

/// Close the stream on unmount.
export function stopStream() {
  active?.stop();
  active = undefined;
}

class UserStream {
  private es: EventSourceLike | undefined;
  private failedOpens = 0;
  private settled = false;
  private stopped = false;
  private startPromise: Promise<void>;
  private settleStart: (p: Promise<void>) => void = () => {};
  private failStart: (e: Error) => void = () => {};

  constructor(private factory: EventSourceFactory) {
    this.startPromise = new Promise<void>((resolve, reject) => {
      this.settleStart = resolve;
      this.failStart = reject;
    });
  }

  start(onSnapshot: (p: Promise<void>) => void): Promise<void> {
    const es = this.factory(streamUrl);
    this.es = es;
    es.onopen = () => {
      if (this.stopped) return;
      this.failedOpens = 0;
      const snapshot = this.snapshot();
      onSnapshot(snapshot);
      if (!this.settled) {
        this.settled = true;
        this.settleStart(snapshot);
      }
    };
    es.onerror = () => {
      if (this.stopped) return;
      this.failedOpens += 1;
      if (this.failedOpens >= maxFailedOpens) this.probe();
      if (!this.settled) {
        this.settled = true;
        this.failStart(new Error("stream open failed"));
      }
    };
    es.onmessage = (e: MessageEvent) => {
      if (this.stopped) return;
      updateSummary(JSON.parse(e.data) as Summary);
    };
    return this.startPromise;
  }

  /// The catch-up snapshot: the full summary the manual refresh uses, so a
  /// (re)connect reconciles anything the stream missed (stream first, then
  /// the snapshot). Retried on failure (spec §7) — upserts keep flowing on
  /// the live stream meanwhile, deletions wait for it.
  private snapshot(): Promise<void> {
    return this.snapshotAttempt(refresh(getShop()?.id), 0);
  }

  /// One bounded snapshot attempt: on failure, wait out the retry spacing
  /// and try again, until the retry budget is spent (then the promise
  /// rejects, which the avatar spinner's `#await` banner surfaces).
  private snapshotAttempt(
    promise: Promise<void>,
    attempt: number,
  ): Promise<void> {
    return promise.catch((e: unknown) => {
      if (this.stopped) throw e;
      if (attempt >= snapshotRetries) throw e;
      return new Promise<void>((resolve) =>
        setTimeout(
          () =>
            resolve(this.snapshotAttempt(refresh(getShop()?.id), attempt + 1)),
          snapshotRetryDelay,
        ),
      );
    });
  }

  /// Liveness probe through `myfetch`, so `checkStatus` centralizes the
  /// 401 → login redirect; a 401 is terminal (it redirects).
  private probe() {
    myfetch("/api/user").catch((e) => {
      if (e instanceof Error) handleError(e);
    });
  }

  stop() {
    this.stopped = true;
    this.es?.close();
    this.es = undefined;
  }
}
