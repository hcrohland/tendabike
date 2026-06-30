<script lang="ts">
  import UsageChips from "../Usage/UsageChips.svelte";
  import { Service } from "../lib/service";
  import { usages } from "../lib/usage";
  import { Part } from "../lib/part";
  import { Usage } from "../lib/usage";
  import { fmtRange, get_days } from "../lib/store";
  import ServiceMenu from "./ServiceMenu.svelte";
  import ServiceBadge from "../Widgets/ServiceBadge.svelte";
  import type { limit_keys } from "../lib/serviceplan";

  interface Props {
    service?: Service | undefined;
    successor?: Service | null;
    part: Part;
    dues?: Partial<Record<limit_keys, { due: number; plan: number }>>;
    children?: import("svelte").Snippet;
  }

  let {
    service = undefined,
    successor = null,
    part,
    dues,
    children,
  }: Props = $props();

  let usage = $derived(
    $usages[successor ? successor.usage : part.usage].sub(
      service ? $usages[service.usage] : new Usage(),
    ),
  );
  let days = $derived(
    get_days(
      service ? service.time : part.purchase,
      successor ? successor.time : new Date(),
    ),
  );
</script>

<span class="text-xs text-text-1 mt-1">
  {days} days
  <ServiceBadge service={dues?.days} />
</span>

{@render children?.()}

<UsageChips {usage} light {dues} />

<div class="flex items-start justify-between gap-2">
  <div class="min-w-0">
    {#if service}
      <span class="flex items-center gap-2">
        <span class="font-medium text-sm">
          {service.name}
        </span>
        <span class="text-xs text-text-1 mt-1">
          · {fmtRange(service ? service.time : part.purchase, successor?.time)}
        </span>
      </span>
      {#if service.notes.length > 0}
        <p class="text-xs text-text-1 mt-1">
          {service.notes}
        </p>
      {/if}
    {:else}
      <span class="text-xs text-text-1 mt-1">
        {#if !successor}
          Since
        {/if}
        {fmtRange(part.purchase, successor?.time)}
      </span>
    {/if}
  </div>
  {#if service}
    <div class="shrink-0">
      <ServiceMenu {service} />
    </div>
  {/if}
</div>
