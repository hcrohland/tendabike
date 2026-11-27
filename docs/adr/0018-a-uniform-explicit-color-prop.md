# ADR-0018: A uniform explicit color prop

## Status

Accepted. The app's neutral color is one uniform `color` prop (`"primary"` or `"secondary"`) that the caller writes explicitly on the neutral *surfaces* — cards and the Tone block — with no context and no hidden state. Chips and neutral buttons take no color: they all render a single uniform fill. This supersedes the surface/tone flip half of [ADR-0017](archive/0017-a-gray-first-two-tone-color-scheme.md) (now in [`archive/`](archive/)). ADR-0017's other accepted decisions stand and are recorded here: the gray-first two-tone palette (primary light, secondary darker) and the vivid status-hue mapping (danger=red, warning/due-soon=yellow, success=green, info/unread=blue).

## Context

ADR-0017 settled the app's color scheme: a single unified, gray-first scheme defined once in the theme (`frontend/src/app.css` `@theme`) — two neutral gray tones, `primary` (light) and `secondary` (darker), with status colors on the standard (vivid) Tailwind hue scales. It also decided the *mechanism* for card/content contrast: a context-based surface/tone flip. A `Card` picked one of the two tones and published it to its children through a context; a nested `Card` flipped to the opposite tone, and content (text, buttons) contrasted the card by reading the published tone and taking the opposite. That mechanism lived in a dedicated module of hand-rolled class-string helpers (`frontend/src/Widgets/tone.ts`: `Tone`, `bg()`, `border()`, `mainText()`, `mutedText()`, `opposite()`, `currentTone()`, `contrastBg()`).

The flip worked, but it was built the wrong way for Tailwind: a component's color depended on invisible ancestor state, so it was hard to follow and not how the component library is meant to be used. The library already expects `primary` and `secondary` to be ordinary color scales that each component reads for its own background, text, and border. And the theme's two gray scales were already shaped to carry the contrast without a flip: `secondary` is a mid-range gray that crosses the full-range `primary` scale around the middle step, so at the surface steps `primary` is the lighter surface and at the fill steps `secondary` is the lighter fill. The contrast the design needs was already a property of the existing values plus each component role's fixed step.

## Decision

**One uniform `color` prop — `"primary"` (the light tone) or `"secondary"` (the darker tone) — on every neutral component, written explicitly by the caller. There is no context, no flip, and no hidden state: a component's color depends only on the prop on it.** The same color value means the same thing on every component; what differs is each component's *role within the same scale*, and that difference is the contrast:

- **Cards and the Tone block render a surface** — background at the scale's surface step (light step in light mode, dark step in dark mode) plus a border from the same scale. `primary` is the light surface; `secondary` the darker one.
- **Chips and neutral buttons render a single uniform fill** — one value, not per-color: `primary-500` in light mode and `secondary-700` in dark mode, with white text. The fill does not vary by the card's color; the contrast comes from the fill sitting between the two card surfaces. Both steps were chosen to stay legible on the lighter (secondary) card surface in their mode. Chips read the fill from the shared mapping; neutral buttons apply it as a background override so they render the same fill whatever library `color` prop they carry.
- **Only surfaces carry a color.** A card and the Tone block take a `color` (their surface). The chips and buttons inside them do not — they render the uniform fill, so there is no per-widget color to match.
- **Only nested cards flip.** A card (or Tone block) inside a card takes the opposite color, authored explicitly by the caller. A top-level card defaults to `secondary`. (Chips and buttons take no color, so there is no per-widget default.)

**The tone module is deleted.** `frontend/src/Widgets/tone.ts` — the context a card published, the opposite/current-tone logic, and the hand-rolled class helpers (background, border, main-text, muted-text, contrast-background) — is gone, and the boolean `light` prop is retired from the card, the chip, and the Tone block in favor of `color`.

**A small shared mapping is the single source of neutral color classes** (`frontend/src/Widgets/color.ts`): a color maps to its surface classes (for cards and the Tone block); the single uniform `fill` (chips) and `fillBg` (the button background override) are shared constants; plus the shared `mainText`/`mutedText` text constants. It is plain data that carries no state and reads no context.

**Standing from ADR-0017.** The gray-first two-tone palette — `primary` (the light tone) and `secondary` (the darker tone), each defined once in the theme with a light-mode step and a dark-mode step, so the same class reads correctly in both modes — and the vivid status-hue mapping: danger (overdue/alert, errors, destructive actions, not-found) = red; warning (due-soon) = yellow; success (ok/healthy, approve/confirm) = green; info (unread, primary call-to-action) = blue; neutral (empty states, dismiss) = the darker tone. Provider brand colors (Strava, intervals.icu) remain inline, outside the theme.

## Rejected alternatives

- **Keep the flip and add `color` beside it.** (The migration's intermediate state: `color` won when set, the flip decided the rest.) Rejected: two ways to do neutral color in one codebase — the flip stays magical, the `light` prop's per-component direction quirks survive, and a component's color still depends on invisible ancestor state. The convention is one prop with one meaning everywhere.
- **Bespoke surface/fill scales per tone.** Define separate theme scales for surfaces and fills instead of reading fixed steps of the two existing scales. Rejected: it doubles the scales and edits the theme's values; the needed contrast is already a property of the two existing scales plus each role's fixed step, so no color values are added, removed, or edited.
- **A shared context that widgets read their card's color from.** Rejected: it re-introduces exactly the invisible-ancestor state the flip is retired for. The caller writes every color explicitly; matching a widget to its card is the caller's job, written out at the call site.
- **Per-color fills (each tone its own fill step, chip/button matching its card).** This was ADR-0018's original fill decision: `primary` a darker fill, `secondary` a lighter one. Rejected after rendering in the browser: the per-color fills read too dark on the light surface in light mode and too light on the dark surface in dark mode, and the two different fills made chips and buttons inconsistent with each other. A single uniform fill, sitting between the two card surfaces, reads cleaner in both modes and keeps every chip and button visually identical.

## Consequences

- `frontend/src/Widgets/color.ts` is the single source of neutral color classes: the color → surface/fill mapping plus the shared text constants. The tone module and the `light` prop are gone; the codebase has exactly one way to do neutral color.
- A component's color is visible at its call site — there is no ancestor state to reason about, and a nested card's flip is readable in the code (the caller writes the opposite color).
- The look shifts in two intended ways: neutral buttons become gray filled (instead of white-on-card), and chips become filled, button-like elements (instead of light surfaces). Both render the same uniform fill, so chips and buttons are visually consistent with each other across the app.
- The theme's color values are unchanged: the fill reads existing steps (`primary-500` light, `secondary-700` dark); status colors and provider brand colors are untouched.
- Tests assert the rendered surface/fill classes of the card, chip, and Tone block — never the mechanism, which no longer exists.
- The frontend coding-standards doc (`frontend/AGENTS.md`) records the uniform `color` convention — surface vs fill, "widgets match, only nested cards flip", and the button color convention — replacing the two-tone flip description.
