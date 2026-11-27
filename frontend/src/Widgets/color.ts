/**
 * The uniform `color` prop's class mapping. A color (`"primary"` or
 * `"secondary"`) maps to its surface classes (background + border, for cards
 * and the Tone block) and its fill classes (dark-step background + white
 * text, for chips — the same fill a flowbite button of that color renders).
 *
 * This is plain data: it carries no state and reads no context. The same
 * color value reads as the light surface / darker fill for `primary` and the
 * darker surface / lighter fill for `secondary` — the contrast comes from each
 * component's role within the same scale, not from a flip.
 */

/** The two neutral colors a card, chip, or Tone block can render in. */
export type Color = "primary" | "secondary";

/** The standard surface border, shared by both colors. */
const surfaceBorder = "border-secondary-400 dark:border-secondary-700";

/** The per-color surface background (no border). */
const primarySurfaceBg = "bg-primary-100 dark:bg-primary-900";
const secondarySurfaceBg = "bg-secondary-100 dark:bg-secondary-900";

interface ColorClasses {
  /** Surface background classes (no border): for a call site composing its own border. */
  surfaceBg: string;
  /** Surface classes (background + standard border): for cards and the Tone block. */
  surface: string;
  /** Fill classes (dark-step background + white text): for chips. */
  fill: string;
}

/** The shared color → classes mapping. */
export const colorClasses: Record<Color, ColorClasses> = {
  primary: {
    surfaceBg: primarySurfaceBg,
    surface: `${primarySurfaceBg} ${surfaceBorder}`,
    fill: "bg-primary-700 dark:bg-primary-600 text-white",
  },
  secondary: {
    surfaceBg: secondarySurfaceBg,
    surface: `${secondarySurfaceBg} ${surfaceBorder}`,
    fill: "bg-secondary-700 dark:bg-secondary-600 text-white",
  },
};

/**
 * The neutral text classes, shared by both colors (text contrast does not
 * depend on the card's color, so these are plain constants, not part of the
 * per-color mapping).
 */

/** The main (high-contrast) text class. */
export const mainText = "text-secondary-900 dark:text-secondary-100";

/** The muted (secondary) text class. */
export const mutedText = "text-secondary-600 dark:text-secondary-300";
