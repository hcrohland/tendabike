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
 * defined once with a light-mode step (100) and a dark-mode step (900), so
 * the same class reads correctly in both modes. The anchor steps carry the
 * app's exact surface grays (primary gray-100/gray-800, secondary
 * gray-300/gray-600). */
export function bg(tone: Tone): string {
  return tone === "primary"
    ? "bg-primary-100 dark:bg-primary-900"
    : "bg-secondary-100 dark:bg-secondary-900";
}

/** The neutral border class: the light-mode step (400) and the dark-mode step
 * (700), defined once so the same class reads correctly in both modes. */
export function border(): string {
  return "border-secondary-400 dark:border-secondary-700";
}

/** The main (high-contrast) text class: the darker tone's anchor steps,
 * light/dark. */
export function mainText(): string {
  return "text-secondary-900 dark:text-secondary-100";
}

/** The muted (secondary) text class: a mid step of the darker tone,
 * light/dark. */
export function mutedText(): string {
  return "text-secondary-600 dark:text-secondary-300";
}

/** The tone the enclosing Card renders in, or `undefined` outside a Card. */
export function currentTone(): Tone | undefined {
  return getContext(ToneContext) as Tone | undefined;
}

/**
 * The background class that contrasts the enclosing Card's tone — content uses
 * the opposite tone. Outside a Card it contrasts the default card tone (the
 * darker one), so it falls back to the light tone.
 */
export function contrastBg(): string {
  const tone = currentTone() ?? "secondary";
  return bg(opposite(tone));
}
