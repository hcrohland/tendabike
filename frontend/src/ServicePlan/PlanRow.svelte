<script lang="ts">
  import ServiceRow from "../Service/ServiceRow.svelte";
  import { attachments } from "../lib/attachment";
  import { Part, parts } from "../lib/part";
  import { services } from "../lib/service";
  import { next_due, ServicePlan } from "../lib/serviceplan";
  import { usages } from "../lib/usage";
  import PlanName from "./PlanName.svelte";
  import ShowMore from "../Widgets/ShowMore.svelte";
  import PlanMenu from "./PlanMenu.svelte";
  import { fmtDate } from "../lib/store";

  interface Props {
    plan: ServicePlan;
    name?: string | null;
  }

  let { plan, name = null }: Props = $props();

  let show_more = $state(false);

  let part = $derived(plan.getpart($parts, $attachments)) as Part;
  let serviceList = $derived(plan.services(part, $services));
  let lastService = $derived(serviceList.at(0));
  let title = "service history";
  let dues: any = $derived(next_due(part, [plan], $services, $usages));
</script>

{#if part}
  <div
    class="rounded-lg border border-gray-200 dark:border-gray-600 bg-surface-2 p-3"
  >
    <!-- Header -->
    <div class="flex items-center justify-between gap-2">
      <div class="flex items-center gap-2 min-w-0">
        <span class="text-sm shrink-0">
          {#if name}
            {@html name}
          {:else}
            <PlanName {plan} />
          {/if}
        </span>
        {#if lastService}
          <span class="text-xs text-gray-500 dark:text-gray-400 shrink-0">
            · since {fmtDate(lastService.time)}
          </span>
        {:else}
          <span class="text-xs text-gray-500 dark:text-gray-400 shrink-0">
            · since purchase
          </span>
        {/if}

        {#if serviceList.length > 0}
          <ShowMore bind:show_more {title} />
        {/if}
      </div>
      <PlanMenu {plan} {name} />
    </div>

    <!-- Service history -->
    <div class="flex flex-col gap-2 mt-3">
      {#if show_more}
        {#each serviceList as service, i (service.id)}
          {@const successor = i > 0 ? serviceList[i - 1] : null}
          <ServiceRow
            {part}
            {service}
            {successor}
            dues={i == 0 ? dues : null}
          />
        {/each}
        <ServiceRow {part} successor={serviceList.at(-1)} />
      {:else}
        <ServiceRow {part} service={serviceList[0]} {dues} />
      {/if}
    </div>
  </div>
{/if}
