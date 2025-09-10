<script lang="ts">
  import { Button, Tooltip } from "@sveltestrap/sveltestrap";
  import { createEventDispatcher } from "svelte";
  const dispatch = createEventDispatcher();

  export let show_more = false;
  export let title: string | undefined = undefined;

  function click() {
    show_more = !show_more;
    dispatch("toggle", show_more);
  }
  let id = "show_more" + Math.floor(Math.random() * 100000);
  let delay = 1000;
</script>

<Button {id} color="light" class="m-0 p-1" on:click={click}>
  {#if show_more}
    −
  {:else}
    +
    {#if title}
      <Tooltip {delay} target={id}>Show {title}</Tooltip>
    {/if}
  {/if}
</Button>
