<Switch id="dispose" bind:checked={dispose}>
  {#if dispose}
    disposing <slot />
  {:else}
    dispose <slot />?
  {/if}
</Switch>
<script lang="ts">
  import Switch from '../Actions/Switch.svelte'
  export let dispose
</script>