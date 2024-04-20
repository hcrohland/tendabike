<script lang="ts">
  import { Button } from "@sveltestrap/sveltestrap";
  import { createEventDispatcher } from "svelte";
  const dispatch = createEventDispatcher();

  export let show_hist = false;

  function click() {
    show_hist = !show_hist;
    dispatch("toggle", show_hist);
  }
</script>

<Button color="light" class="m-0 p-1" on:click={click}>
  {#if show_hist}
    −
  {:else}
    +
  {/if}
</Button>
