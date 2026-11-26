<script lang="ts">
  import { Button } from "flowbite-svelte";
  import { m } from "../../paraglide/messages";
  import type { Color } from "./color.js";

  interface Props {
    open: boolean;
    label?: string;
    color?: Color;
  }

  let {
    open = $bindable(false),
    label = undefined,
    color = "secondary",
  }: Props = $props();

  function onclick() {
    open = false;
  }
</script>

<div class="flex justify-end w-full gap-2">
  <!-- <div class="ms-auto"> -->
  {#if label}
    <Button {onclick} {color}>{m.action_cancel()}</Button>
    <Button type="submit" value="commit" {color}>
      {label}
    </Button>
  {:else}
    <Button {onclick} {color}>{m.action_close()}</Button>
  {/if}
</div>
