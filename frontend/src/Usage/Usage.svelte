<script lang="ts">
  import { link } from "svelte-spa-router";
  import { fmtNumber, fmtSeconds } from "../lib/store";
  import { Usage, usages } from "../lib/usage";
  import { TableBodyCell, TableHeadCell } from "flowbite-svelte";

  export let header = false;
  export let id: string | undefined = undefined;
  export let usage: Usage = new Usage();
  export let ref: string | number | undefined = undefined;

  $: if (id && $usages[id]) usage = $usages[id];
</script>

{#if !header}
  <TableBodyCell class="text-end">
    {#if ref}
      <a class="text-reset" use:link href={"/activities/" + ref}>
        {fmtNumber(usage.count)}
      </a>
    {:else}
      {fmtNumber(usage.count)}
    {/if}
  </TableBodyCell>
  <TableBodyCell class="text-end">
    {fmtSeconds(usage.time)}
  </TableBodyCell>
  <TableBodyCell class="text-end">
    {fmtNumber(Math.round((usage.distance || 0) / 1000))}
  </TableBodyCell>
  <TableBodyCell class="text-end">
    {fmtNumber(usage.climb)}
  </TableBodyCell>
  <TableBodyCell class="text-end">
    {fmtNumber(usage.descend)}
  </TableBodyCell>
  <TableBodyCell class="text-end">
    {fmtNumber(usage.energy)}
  </TableBodyCell>
{:else}
  <TableHeadCell scope="col" title="Number of activities">Rides</TableHeadCell>
  <TableHeadCell class="text-end" scope="col" title="Time (h)">
    Time
  </TableHeadCell>
  <TableHeadCell class="text-end" scope="col" title="Distance (km)">
    Distance
  </TableHeadCell>
  <TableHeadCell class="text-end" scope="col" title="Climb (m)">
    Climb
  </TableHeadCell>
  <TableHeadCell class="text-end" scope="col" title="Descend (m)">
    Descend
  </TableHeadCell>
  <TableHeadCell class="text-end" scope="col" title="Energy (kJ)">
    Energy
  </TableHeadCell>
{/if}
