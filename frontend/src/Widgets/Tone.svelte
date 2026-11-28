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
</script>

<div
  class={"rounded-lg " + (extraClass ? extraClass : "")}
  class:surface-primary={color === "primary"}
  class:surface-secondary={color === "secondary"}
>
  {@render children()}
</div>
