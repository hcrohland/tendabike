<script lang="ts">
  import PlanRow from "./PlanRow.svelte";
  import { parts } from "../lib/part";
  import { plans, ServicePlan } from "../lib/serviceplan";

  export let plan: ServicePlan;
</script>

{#if plan.part}
  <PlanRow {plan} />
{:else}
  <PlanRow {plan} />
  {#each plan.gears($parts, Object.values($plans)) as part}
    {@const p = new ServicePlan({ ...plan, part: part.id })}
    <PlanRow plan={p} name={part.partLink()} />
  {/each}
{/if}
