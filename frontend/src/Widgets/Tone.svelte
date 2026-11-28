<script lang="ts">
  import { colorClasses, type Color } from "./color.js";

  let {
    color,
    class: extraClass = "",
    children,
  }: {
    color: Color;
    class?: string;
    children: import("svelte").Snippet;
  } = $props();

  // A Tone block is a nested surface: it renders the surface (background +
  // border) of its color. The caller writes it explicitly — a Tone inside a
  // card flips relative to that card, like any nested card.
  const surface = $derived(colorClasses[color].surface);
</script>

<div class={"rounded-lg " + surface + (extraClass ? " " + extraClass : "")}>
  {@render children()}
</div>
