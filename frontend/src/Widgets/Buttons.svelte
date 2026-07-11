<script lang="ts">
  import { Button } from "flowbite-svelte";
  import { m } from "../../paraglide/messages";

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
  <Button {onclick} color="alternative">{m.action_cancel()}</Button>
  <Button type="submit" value="commit" class="float-end" {disabled}>
    {label}
  </Button>
{:else}
  <Button {onclick}>{m.action_close()}</Button>
{/if}
