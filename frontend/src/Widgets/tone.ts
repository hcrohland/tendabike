import { getContext } from "svelte";

/**
 * The surface/tone flip. The app's neutral system is two tones: `primary` is
 * the light tone (surface-1) and `secondary` the darker tone (surface-2). A
 * Card renders one tone and provides it to its children through a context; a
 * nested Card flips to the opposite tone, and content (text, buttons) contrasts
 * the Card by reading the current tone and using the opposite.
 */

/** The two neutral tones a Card can render in. */
export type Tone = "primary" | "secondary";

/** Context key under which a Card publishes its tone to its children. */
export const ToneContext = Symbol("tone");

/** The tone opposite `tone` — the card/content flip. */
export function opposite(tone: Tone): Tone {
  return tone === "primary" ? "secondary" : "primary";
}

/** The surface background class for `tone`. */
export function bg(tone: Tone): string {
  return tone === "primary" ? "bg-surface-1" : "bg-surface-2";
}

/** The tone the enclosing Card renders in, or `undefined` outside a Card. */
export function currentTone(): Tone | undefined {
  return getContext(ToneContext) as Tone | undefined;
}

/**
 * The background class that contrasts the enclosing Card's tone — content uses
 * the opposite tone. Falls back to the darker tone outside a Card.
 */
export function contrastBg(): string {
  const tone = currentTone();
  return tone ? bg(opposite(tone)) : "bg-surface-2";
}
