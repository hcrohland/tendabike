<script lang="ts">
  import PlanRow from "./PlanRow.svelte";
  import PlanName from "./PlanName.svelte";
  import { attachments } from "../lib/attachment";
  import { parts } from "../lib/part";
  import { plans, ServicePlan, gearsForPlan } from "../lib/serviceplan";
  import * as m from "../../paraglide/messages";
  import Menu from "../Widgets/Menu.svelte";
  import { DropdownItem } from "flowbite-svelte";
  import { actions } from "../Widgets/Actions.svelte";
  import Card from "../Widgets/Card.svelte";

  interface Props {
    plan: ServicePlan;
  }

  let { plan }: Props = $props();

  let gears = $derived(gearsForPlan(plan, $parts, $attachments, $plans));
</script>

<Card>
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
</Card>
