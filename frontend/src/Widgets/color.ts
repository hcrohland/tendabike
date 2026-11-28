/**
 * The uniform `color` prop's type. The theme owns the complete neutral
 * style — a surface role class per color (background + text + border) and
 * the single uniform fill class (ADR-0019) — so this module carries no
 * class strings: a surface names its one surface role class, and a chip or
 * neutral button names the fill.
 */

/** The two neutral colors a card or Tone block can render in. */
export type Color = "primary" | "secondary";
