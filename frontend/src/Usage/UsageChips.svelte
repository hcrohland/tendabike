<script lang="ts">
  import type { limit_keys } from "../lib/serviceplan";
  import { fmtNumber, fmtSeconds } from "../lib/store";
  import { Usage, usages } from "../lib/usage";
  import Chip from "../Widgets/Chip.svelte";

  export let id: string | undefined = undefined;
  export let usage: Usage = new Usage();
  export let ref: string | number | undefined = undefined;
  export let light = false;
  export let gridclass = "md:grid-cols-6 grid-cols-3";
  export let dues: Partial<Record<limit_keys, { due: number; plan: number }>> =
    {};

  $: if (id && $usages[id]) usage = $usages[id];

  $: ridesHref = ref ? "/activities/" + ref : undefined;
</script>

<div class={"grid gap-2 m-2 " + gridclass}>
  <Chip
    value={fmtNumber(usage.count)}
    label="rides"
    href={ridesHref}
    {light}
    service={dues?.rides}
  />
  <Chip
    value={fmtSeconds(usage.time)}
    label="h"
    {light}
    service={dues?.hours}
  />
  <Chip
    value={fmtNumber(Math.round((usage.distance || 0) / 1000))}
    label="km"
    {light}
    service={dues?.km}
  />
  <Chip
    value={fmtNumber(usage.climb)}
    label="↑m"
    {light}
    service={dues?.climb}
  />
  <Chip
    value={fmtNumber(usage.descend)}
    label="↓m"
    {light}
    service={dues?.descend}
  />
  {#if usage.energy > 0}
    <Chip
      value={fmtNumber(usage.energy)}
      label="kJ"
      {light}
      service={dues?.kJ}
    />
  {/if}
</div>
