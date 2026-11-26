<script lang="ts">
  import { m } from "../../paraglide/messages";
  import type { Due, limit_keys } from "../lib/serviceplan";
  import { fmtNumber, fmtSeconds } from "../lib/utils";
  import { Usage, usages } from "../lib/usage";
  import Chip from "../Widgets/Chip.svelte";
  import { type Color } from "../Widgets/color.js";

  let {
    id,
    usage = $bindable(),
    ref,
    color = "secondary",
    gridclass = "md:grid-cols-6 grid-cols-3",
    due_list,
  }: {
    id?: string;
    usage?: Usage;
    ref?: string | number;
    color?: Color;
    gridclass?: string;
    due_list?: Partial<Record<limit_keys, Due>>;
  } = $props();

  let currentUsage = $derived.by(() => {
    if (usage) return usage;
    if (id && usages[id]) return usages[id];
    return new Usage();
  });

  let ridesHref = $derived.by(() => (ref ? "/activities/" + ref : undefined));
</script>

<div class={"grid gap-2 m-2 " + gridclass}>
  <Chip
    value={fmtNumber(currentUsage.count)}
    label={m.usage_rides()}
    href={ridesHref}
    {color}
    due={due_list?.rides}
  />
  <Chip
    value={fmtSeconds(currentUsage.time)}
    label="h"
    {color}
    due={due_list?.hours}
  />
  <Chip
    value={fmtNumber(Math.round((currentUsage.distance || 0) / 1000))}
    label="km"
    {color}
    due={due_list?.km}
  />
  <Chip
    value={fmtNumber(currentUsage.climb)}
    label="↑m"
    {color}
    due={due_list?.climb}
  />
  <Chip
    value={fmtNumber(currentUsage.descend)}
    label="↓m"
    {color}
    due={due_list?.descend}
  />
  {#if currentUsage.energy > 0}
    <Chip
      value={fmtNumber(currentUsage.energy)}
      label="kJ"
      {color}
      due={due_list?.kJ}
    />
  {/if}
</div>
