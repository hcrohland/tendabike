<script lang="ts">
  import { colorClasses, type Color } from "./color.js";
  import { contrastBg } from "./tone.js";

  let {
    color = undefined,
    class: extraClass = "",
    children,
  }: {
    color?: Color;
    class?: string;
    children: import("svelte").Snippet;
  } = $props();

  // Content contrasts the enclosing Card: it renders in the opposite tone.
  // Read during init so `getContext` runs in the component's setup phase.
  const contrast = contrastBg();
  // A `color` renders the Tone's own surface for that color; without one it
  // keeps the old contrast behavior.
  const background = $derived(color ? colorClasses[color].surface : contrast);
</script>

<div class={"rounded-lg " + background + (extraClass ? " " + extraClass : "")}>
  {@render children()}
</div>
