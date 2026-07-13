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

<div class="flex justify-end w-full gap-2">
  <!-- <div class="ms-auto"> -->
  {#if label}
    <Button {onclick} color="alternative">{m.action_cancel()}</Button>
    <Button type="submit" value="commit" color="gray">
      {label}
    </Button>
  {:else}
    <Button {onclick}>{m.action_close()}</Button>
  {/if}
</div>
