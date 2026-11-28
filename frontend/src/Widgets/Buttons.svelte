<script lang="ts">
  import { m } from "../../paraglide/messages";
  import NeutralButton from "./NeutralButton.svelte";

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
  {#if label}
    <NeutralButton {onclick}>{m.action_cancel()}</NeutralButton>
    <NeutralButton type="submit" value="commit">{label}</NeutralButton>
  {:else}
    <NeutralButton {onclick}>{m.action_close()}</NeutralButton>
  {/if}
</div>
