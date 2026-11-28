<script lang="ts">
  import NeutralButton from "./NeutralButton.svelte";

  let {
    onclick = undefined,
    disabled = false,
    class: extraClass = "",
    children,
  }: {
    onclick?: (e: MouseEvent) => void;
    disabled?: boolean;
    class?: string;
    children: import("svelte").Snippet;
  } = $props();
</script>

<NeutralButton
  size="xs"
  {disabled}
  {onclick}
  class={"ms-2 " + (extraClass ? extraClass : "")}
>
  {@render children()}
</NeutralButton>
