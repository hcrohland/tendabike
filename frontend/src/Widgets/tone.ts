import { getContext } from "svelte";

/**
 * The surface/tone flip. The app's neutral system is two tones: `primary` is
 * the light tone and `secondary` the darker tone. A Card renders one tone and
 * provides it to its children through a context; a nested Card flips to the
 * opposite tone, and content (text, buttons) contrasts the Card by reading the
 * current tone and using the opposite.
 */

/** The two neutral tones a Card can render in. */
export type Tone = "primary" | "secondary";

/** Context key under which a Card publishes its tone to its children. */
export const ToneContext = Symbol("tone");

/** The tone opposite `tone` — the card/content flip. */
export function opposite(tone: Tone): Tone {
  return tone === "primary" ? "secondary" : "primary";
}

/** The surface background class for `tone`: the two neutral tones, each
 * defined once with a light-mode step (100/300) and a dark-mode step
 * (800/600), so the same class reads correctly in both modes. */
export function bg(tone: Tone): string {
  return tone === "primary"
    ? "bg-primary-100 dark:bg-primary-800"
    : "bg-secondary-300 dark:bg-secondary-600";
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
  return tone ? bg(opposite(tone)) : "bg-secondary-300 dark:bg-secondary-600";
}
