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

/** The class pairs a color renders as, per component role. */
export interface ColorClasses {
  /** Surface classes (background + border): for cards and the Tone block. */
  surface: string;
  /** Fill classes (dark-step background + white text): for chips. */
  fill: string;
}

/** The shared color → classes mapping. */
export const colorClasses: Record<Color, ColorClasses> = {
  primary: {
    surface:
      "bg-primary-100 dark:bg-primary-900 border-secondary-400 dark:border-secondary-700",
    fill: "bg-primary-700 dark:bg-primary-600 text-white",
  },
  secondary: {
    surface:
      "bg-secondary-100 dark:bg-secondary-900 border-secondary-400 dark:border-secondary-700",
    fill: "bg-secondary-700 dark:bg-secondary-600 text-white",
  },
};
