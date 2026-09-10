<script lang="ts">
  import Buttons from "./Buttons.svelte";

  let { label = undefined }: { label?: string } = $props();
  let open = $state(true);
</script>

<Buttons bind:open {label} />

<div role="status" data-testid="open-state">open: {String(open)}</div>
