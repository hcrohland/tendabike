<script lang="ts">
  import { m } from "../../paraglide/messages";
  import type { limit_keys } from "../lib/serviceplan";
  import { fmtNumber, fmtSeconds } from "../lib/store";
  import { Usage, usages } from "../lib/usage";
  import Chip from "../Widgets/Chip.svelte";

  let {
    id,
    usage = $bindable(),
    ref,
    light = false,
    gridclass = "md:grid-cols-6 grid-cols-3",
    dues,
  }: {
    id?: string;
    usage?: Usage;
    ref?: string | number;
    light?: boolean;
    gridclass?: string;
    dues?: Partial<Record<limit_keys, { due: number; plan: number }>>;
  } = $props();

  let currentUsage = $derived.by(() => {
    if (usage) return usage;
    if (id && $usages[id]) return $usages[id];
    return new Usage();
  });

  let ridesHref = $derived.by(() => (ref ? "/activities/" + ref : undefined));
</script>

<div class={"grid gap-2 m-2 " + gridclass}>
  <Chip
    value={fmtNumber(currentUsage.count)}
    label={m.usage_rides()}
    href={ridesHref}
    {light}
    service={dues?.rides}
  />
  <Chip
    value={fmtSeconds(currentUsage.time)}
    label="h"
    {light}
    service={dues?.hours}
  />
  <Chip
    value={fmtNumber(Math.round((currentUsage.distance || 0) / 1000))}
    label="km"
    {light}
    service={dues?.km}
  />
  <Chip
    value={fmtNumber(currentUsage.climb)}
    label="↑m"
    {light}
    service={dues?.climb}
  />
  <Chip
    value={fmtNumber(currentUsage.descend)}
    label="↓m"
    {light}
    service={dues?.descend}
  />
  {#if currentUsage.energy > 0}
    <Chip
      value={fmtNumber(currentUsage.energy)}
      label="kJ"
      {light}
      service={dues?.kJ}
    />
  {/if}
</div>
