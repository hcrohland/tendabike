<script lang="ts">
  import {
    Button,
    ModalFooter,
    Spinner,
  } from '@sveltestrap/sveltestrap';
  
  export let toggle;
  export let action;
  export let disabled = false;
  export let button
  let promise;

</script>

<ModalFooter>
  {#if button}
    <Button color="secondary" on:click={toggle}>Cancel</Button>
    <Button color="primary" {disabled} on:click={() => (promise = action())}>
      {#await promise}
        <Spinner />
      {:then} 
        {button}
      {/await}
    </Button>
  {:else}
    <Button color="primary" on:click={toggle}>Close</Button>
  {/if}
</ModalFooter>