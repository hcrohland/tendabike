<script lang="ts">
  import { contrastBg } from "./tone.js";

  let {
    class: extraClass = "",
    children,
  }: {
    class?: string;
    children: import("svelte").Snippet;
  } = $props();

  // Content contrasts the enclosing Card: it renders in the opposite tone.
  // Read during init so `getContext` runs in the component's setup phase.
  const background = contrastBg();
</script>

<div class={"rounded-lg " + background + (extraClass ? " " + extraClass : "")}>
  {@render children()}
</div>
