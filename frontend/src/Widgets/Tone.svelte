<script lang="ts">
  import type { Color } from "./color.js";

  let {
    color,
    class: extraClass = "",
    children,
  }: {
    color: Color;
    class?: string;
    children: import("svelte").Snippet;
  } = $props();

  // A Tone block is a nested surface: it renders the theme's surface role
  // class of its color (background + text + border). The caller writes it
  // explicitly — a Tone inside a card flips relative to that card, like any
  // nested card.
</script>

<div
  class={"rounded-lg " + (extraClass ? extraClass : "")}
  class:surface-primary={color === "primary"}
  class:surface-secondary={color === "secondary"}
>
  {@render children()}
</div>
