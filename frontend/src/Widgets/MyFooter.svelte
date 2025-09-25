<script lang="ts">
  import { Button, ModalFooter, Spinner } from "flowbite-svelte";

  export let toggle: () => void;
  export let action: () => Promise<void>;
  export let disabled = false;
  export let label: any;
  let promise: Promise<void>;
</script>

<ModalFooter>
  {#if label}
    <Button type="button" on:click={toggle}>Cancel</Button>
    <Button color="primary" {disabled} on:click={() => (promise = action())}>
      {#await promise}
        <Spinner />
      {:then}
        {label}
      {/await}
    </Button>
  {:else}
    <Button color="primary" on:click={toggle}>Close</Button>
  {/if}
</ModalFooter>
