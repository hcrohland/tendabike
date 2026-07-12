<script lang="ts">
  import { Button } from "flowbite-svelte";
  import { m } from "../../paraglide/messages";

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
  <Button {onclick} color="alternative">{m.action_cancel()}</Button>
  <Button type="submit" class="border border-border-subtle hover:bg-surface-2">
    {label}
  </Button>
{:else}
  <Button {onclick}>{m.action_close()}</Button>
{/if}
