<script lang="ts">
  import { Input } from '@sveltestrap/sveltestrap';

  export let timezone = Intl.DateTimeFormat().resolvedOptions().timeZone;

</script>

<Input type="select" id="tzpicker" bind:value={timezone}>
  {#each Intl.supportedValuesOf('timeZone') as value }
    <option {value} selected={value == timezone}> {value} </option>
  {/each}
</Input>