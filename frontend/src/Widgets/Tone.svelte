<script lang="ts">
  import { bg, currentTone, opposite } from "./tone.js";

  let {
    class: extraClass = "",
    children,
  }: {
    class?: string;
    children: import("svelte").Snippet;
  } = $props();

  // Content contrasts the enclosing Card: it renders in the opposite tone.
  // Read during init so `getContext` runs in the component's setup phase.
  const tone = currentTone();
  const background = tone ? bg(opposite(tone)) : "bg-surface-2";
</script>

<div class={"rounded-lg " + background + (extraClass ? " " + extraClass : "")}>
  {@render children()}
</div>
