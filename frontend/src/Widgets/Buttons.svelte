<script lang="ts">
  import { Button } from "flowbite-svelte";

  interface Props {
    open: boolean;
    label?: string;
  }

  let { open = $bindable(false), label = undefined }: Props = $props();

  function onclick() {
    open = false;
  }
</script>

{#if label}
  <Button {onclick} color="alternative">Cancel</Button>
  <Button type="submit" value="commit" class="float-end">
    {label}
  </Button>
{:else}
  <Button {onclick}>Close</Button>
{/if}
