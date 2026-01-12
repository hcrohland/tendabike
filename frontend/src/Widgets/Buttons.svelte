<script lang="ts">
  import { Button } from "flowbite-svelte";

  interface Props {
    open: boolean;
    label?: string;
    disabled?: boolean;
  }

  let {
    open = $bindable(false),
    label = undefined,
    disabled = false,
  }: Props = $props();

  function onclick() {
    open = false;
  }
</script>

{#if label}
  <Button {onclick} color="alternative">Cancel</Button>
  <Button type="submit" value="commit" class="float-end" {disabled}>
    {label}
  </Button>
{:else}
  <Button {onclick}>Close</Button>
{/if}
