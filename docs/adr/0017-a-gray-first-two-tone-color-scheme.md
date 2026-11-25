# ADR-0017: A gray-first two-tone color scheme

## Status

Accepted. The app's color is a single unified scheme defined once in the frontend theme: a gray-first neutral system of two tones that form a card/content flip, plus four muted status hues. It replaces the ad-hoc color application that preceded it — scattered flowbite `color` props, raw hue utilities, and the separate `surface`/`border`/`text` tokens — and resolves the state divergences that came with it.

## Context

The app's colors were applied ad-hoc: semantic color was scattered across flowbite `color`-prop strings, a few raw hue utilities, and two local mappings that had already diverged — the "due soon" state rendered yellow on metric chips but amber on the plan badge, the unread-notification count was red on the bell but blue in the panel, and green meant both "healthy/ok" and "owned by someone else." There was no single source of truth for color, and the `primary`/`secondary` scales the component library expects were left undefined, so a couple of components rendered against missing colors. The result was a mostly-gray, calm design with color used unpredictably and a handful of states that looked different in different places.

## Decision

**A single, unified, gray-first color scheme defined once in the theme** (`frontend/src/app.css` `@theme`), with three parts:

- **Two neutral tones.** The neutral system is reduced to two gray tones — `primary` (the light tone) and `secondary` (the darker tone) — each defined once with a light-mode step and a dark-mode step (a light-mode background at step 100, a dark-mode background at step 900), so the same class reads correctly in both modes. The previous separate `surface`/`border`/`text` tokens are retired in favor of these two tones; the surface anchor grays are preserved and the in-between steps form one consistent cool-gray family (Tailwind gray), so the look does not change.

- **A surface/tone flip.** A `Card` determines its tone and publishes it to its children through a context (`frontend/src/Widgets/tone.ts`); a nested `Card` flips to the opposite tone, and content (text, buttons) contrasts the card by taking the opposite tone. The flip is the single source of card/content contrast — it replaces the ad-hoc manual background overrides. Components that already carry a `light` prop (the card, the chip) keep it, with each component's existing `light` direction preserved so the look does not shift.

- **Bespoke muted status hues via theme scale overrides.** The only color in the app is a small set of muted status hues (danger, warning, success, info), defined by *overriding the standard Tailwind hue scales* (red, amber, emerald, blue) in the theme. Overriding the scales — rather than introducing new named scales — means the whole app picks up the muted values: both raw hue utilities and the component library's own colored components. The muted values are chosen so color reads as a subtle signal and the app stays calm; `green` is aliased to the success (emerald) scale.

The semantic mapping is: danger (overdue/alert, errors, destructive actions, not-found) = red; warning (due-soon) = amber; success (ok/healthy, approve/confirm) = emerald; info (unread, primary call-to-action) = blue; neutral (empty states, dismiss) = the darker tone. Provider brand colors (Strava, intervals.icu) stay inline, outside the theme.

## Rejected alternatives

- **Tailwind-default hues.** Keep the standard, vivid hue scales and just apply them. Rejected: the defaults are too vivid for a calm, gray-first design — color would shout rather than signal, and the app would lose its unified, muted character.
- **Separate named semantic scales.** Define new theme scales named `danger`/`warning`/`success`/`info` instead of overriding the standard hue scales. Rejected: it doubles the scales and splits the source of truth — raw hue utilities and the component library's colored components would keep reading the *un-muted* defaults while the app read the semantic scales, re-introducing the very divergence this decision removes. Overriding the standard scales keeps one source of truth that everything, including the library, reads.

## Consequences

- `frontend/src/app.css` `@theme` is the single source of all app color: two neutral tones plus four muted status scales (green aliased to emerald). There are no per-mode values; light/dark mode is a step choice (100 vs 900), not a separate scale.
- Card/content contrast is automatic: a `Card` publishes its tone, a nested `Card` flips, and content contrasts via the opposite tone (`tone.ts`: `Tone`, `bg()`, `opposite()`, `currentTone()`, `contrastBg()`). The ad-hoc manual background overrides are gone.
- The known divergences are resolved by construction: due-soon is amber everywhere, the unread count is blue everywhere, and the ownership badge uses the neutral/darker tone (no longer green).
- The `light` prop keeps working on the card and the chip, with each component's existing direction preserved, so existing code does not break and the look does not shift.
- State modifiers (disposed, seen, disabled) are expressed as opacity composed on top of the tone, orthogonal to which tone is in use.
- Provider brand colors remain inline values, outside the themed system.
- The frontend coding-standards doc (`frontend/AGENTS.md`) records the two-tone flip as the color-scheme and button-color convention, so future changes follow the system.
