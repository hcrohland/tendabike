<script lang="ts">
  import { parts } from "../lib/part";
  import { filterValues } from "../lib/mapable";
  import { Type } from "../lib/types";
  import { Select } from "flowbite-svelte";

  export let type: Type;
  export let part: number | undefined;
  export let none = false;

  $: gears = filterValues($parts, (p) => type.main == p.what && !p.disposed_at);
</script>

<Select required bind:value={part}>
  {#if none}
    <option value={undefined}> -- None -- </option>
  {/if}
  {#each gears as gear}
    <option value={gear.id}>{gear.name}</option>
  {/each}
</Select>
