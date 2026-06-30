<script lang="ts">
  import PlanRow from "./PlanRow.svelte";
  import PlanMenu from "./PlanMenu.svelte";
  import PlanName from "./PlanName.svelte";
  import { Part, parts } from "../lib/part";
  import { plans, ServicePlan } from "../lib/serviceplan";

  interface Props {
    plan: ServicePlan;
    part?: Part;
  }

  let { plan, part }: Props = $props();

  let gears = $derived(plan.gears($parts, Object.values($plans)));
</script>

{#if part}
  <PlanRow {plan} />
{:else}
  <div class="rounded-lg border border-border-subtle bg-surface-2 p-3">
    <!-- Template header -->
    <div class="flex items-center justify-between gap-2 mb-2">
      <span class="font-medium text-sm"><PlanName {plan} /></span>
      <PlanMenu {plan} />
    </div>

    <!-- Per-gear rows nested inside -->
    {#if gears.length > 0}
      <div class="flex flex-col gap-2">
        {#each gears as part}
          {@const p = new ServicePlan({ ...plan, part: part.id })}
          <PlanRow plan={p} name={part.partLink()} />
        {/each}
      </div>
    {:else}
      <p class="text-xs text-text-1">No bikes match this plan.</p>
    {/if}
  </div>
{/if}
