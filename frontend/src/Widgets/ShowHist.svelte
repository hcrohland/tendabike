<button class="btn" on:click={click}>
  {#if show_hist}
    &#9650;
  {:else}
    &#9660;
    <slot />
  {/if}
</button>

<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  const dispatch = createEventDispatcher()
  
  export let show_hist = false

  function click() {
    show_hist = !show_hist
    dispatch('toggle', show_hist)
  }
</script>