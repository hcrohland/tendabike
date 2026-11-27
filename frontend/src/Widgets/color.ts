/**
 * The uniform `color` prop's class mapping. A color (`"primary"` or
 * `"secondary"`) maps to its surface classes (background + border) for cards
 * and the Tone block. Chips and neutral buttons do NOT vary by color: they all
 * render the single uniform {@link fill} — the contrast comes from the fill
 * sitting between the two card surfaces, not from a per-color step.
 *
 * This is plain data: it carries no state and reads no context.
 */

/** The two neutral colors a card or Tone block can render in. */
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
}

/** The shared color → surface classes mapping. */
export const colorClasses: Record<Color, ColorClasses> = {
  primary: {
    surfaceBg: primarySurfaceBg,
    surface: `${primarySurfaceBg} ${surfaceBorder}`,
  },
  secondary: {
    surfaceBg: secondarySurfaceBg,
    surface: `${secondarySurfaceBg} ${surfaceBorder}`,
  },
};

/**
 * The single uniform fill for every chip and neutral button: a dark-step
 * background + white text. It is one value, not per-color — light mode sits at
 * `primary-500` and dark mode at `secondary-700`, both chosen to stay visible
 * on the lighter (secondary) card surface in their mode. (ADR-0019.)
 */
export const fill = "bg-primary-500 dark:bg-secondary-700 text-white";

/**
 * The button background override: the same fill, with `!` so it reliably wins
 * over flowbite's per-color button background. Covers base + hover in both
 * modes so a neutral `<Button>` renders the uniform fill regardless of the
 * `color` prop it is given.
 */
export const fillBg =
  "!bg-primary-500 hover:!bg-primary-600 dark:!bg-secondary-700 dark:hover:!bg-secondary-800";

/**
 * The neutral text classes, shared by both colors (text contrast does not
 * depend on the card's color, so these are plain constants, not part of the
 * per-color mapping).
 */

/** The main (high-contrast) text class. */
export const mainText = "text-secondary-900 dark:text-secondary-100";

/** The muted (secondary) text class. */
export const mutedText = "text-secondary-600 dark:text-secondary-300";
