<script lang="ts">
  import PlanRow from "./PlanRow.svelte";
  import PlanName from "./PlanName.svelte";
  import { parts } from "../lib/part";
  import { plans, ServicePlan } from "../lib/serviceplan";
  import * as m from "../../paraglide/messages";
  import Menu from "../Widgets/Menu.svelte";
  import { DropdownItem } from "flowbite-svelte";
  import { actions } from "../Widgets/Actions.svelte";

  interface Props {
    plan: ServicePlan;
  }

  let { plan }: Props = $props();

  let gears = $derived(plan.gears($parts, Object.values($plans)));
</script>

<div
  class="rounded-lg border border-border-subtle bg-surface-2 0 p-0 md:m-2 md:p-2"
>
  <!-- Template header -->
  <div class="flex items-center justify-between gap-1 md:gap-2 p-1">
    <span class="font-medium text-sm"><PlanName {plan} /></span>
    <Menu>
      <DropdownItem onclick={() => $actions.updatePlan(plan)}>
        {m.planmenu_change()}
      </DropdownItem>
      <DropdownItem onclick={() => $actions.deletePlan(plan)}>
        {m.planmenu_delete()}
      </DropdownItem>
    </Menu>
  </div>

  <!-- Per-gear rows nested inside -->
  {#if gears.length > 0}
    <div class="flex flex-col gap-1 md:gap-2">
      {#each gears as gear}
        <PlanRow {plan} {gear} />
      {/each}
    </div>
  {:else}
    <p class="text-xs text-text-1">{m.planblock_no_bikes()}</p>
  {/if}
</div>
