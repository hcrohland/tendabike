<script lang="ts">
  import Switch from "../Widgets/Switch.svelte";
  export let dispose: boolean;
</script>

<Switch id="dispose" bind:checked={dispose}>
  {#if dispose}
    disposing <slot />
  {:else}
    dispose <slot />?
  {/if}
</Switch>
