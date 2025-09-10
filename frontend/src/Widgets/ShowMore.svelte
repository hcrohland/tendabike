<script lang="ts">
  import { Button } from "@sveltestrap/sveltestrap";
  import { createEventDispatcher } from "svelte";
  const dispatch = createEventDispatcher();

  export let show_more = false;

  function click() {
    show_more = !show_more;
    dispatch("toggle", show_more);
  }
</script>

<Button color="light" class="m-0 p-1" on:click={click}>
  {#if show_more}
    −
  {:else}
    +
  {/if}
</Button>
