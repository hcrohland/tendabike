<script lang="ts">
  import { fmtNumber, fmtSeconds } from "../lib/store";
  import { Usage, usages } from "../lib/usage";
  import Chip from "../Widgets/Chip.svelte";

  export let id: string | undefined = undefined;
  export let usage: Usage = new Usage();
  export let ref: string | number | undefined = undefined;
  export let light = false;
  export let gridclass = "md:grid-cols-6 grid-cols-3";

  $: if (id && $usages[id]) usage = $usages[id];

  $: ridesHref = ref ? "/activities/" + ref : undefined;
</script>

<div class={"grid gap-2 m-2 " + gridclass}>
  <Chip value={fmtNumber(usage.count)} label="rides" href={ridesHref} {light} />
  <Chip value={fmtSeconds(usage.time)} label="h" {light} />
  <Chip
    value={fmtNumber(Math.round((usage.distance || 0) / 1000))}
    label="km"
    {light}
  />
  <Chip value={fmtNumber(usage.climb)} label="↑m" {light} />
  <Chip value={fmtNumber(usage.descend)} label="↓m" {light} />
  {#if usage.energy > 0}
    <Chip value={fmtNumber(usage.energy)} label="kJ" {light} />
  {/if}
</div>
