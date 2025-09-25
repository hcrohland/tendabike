<script lang="ts">
  import { Button } from "flowbite-svelte";

  export let toggle: () => void;
  export let disabled = false;
  export let label: any;
</script>

{#if label}
  <Button type="button" on:click={toggle}>Cancel</Button>
  <Button type="submit" color="primary" {disabled}>
    {label}
  </Button>
{:else}
  <Button type="button" color="primary" on:click={toggle}>Close</Button>
{/if}
