<script lang="ts">
  import {
    Dropdown,
    DropdownDivider,
    DropdownItem,
    TableBodyCell,
    TableBodyRow,
  } from "flowbite-svelte";
  import ServiceRow from "../Service/ServiceRow.svelte";
  import { attachments } from "../lib/attachment";
  import { Part, parts } from "../lib/part";
  import { services } from "../lib/service";
  import { ServicePlan } from "../lib/serviceplan";
  import { usages } from "../lib/usage";
  import PlanCell from "./PlanCell.svelte";
  import PlanName from "./PlanName.svelte";
  import ShowMore from "../Widgets/ShowMore.svelte";
  import { ChevronDownOutline } from "flowbite-svelte-icons";
  import { actions } from "../Widgets/Actions.svelte";

  interface Props {
    plan: ServicePlan;
    name?: string | null;
  }

  let { plan, name = null }: Props = $props();

  let show_more = $state(false);

  let part = $derived(plan.getpart($parts, $attachments)) as Part;
  let serviceList = $derived(plan.services(part, $services));
  let due = $derived(plan.due(part, serviceList.at(0), $usages));
  let title = "service history";
</script>

<TableBodyRow>
  <TableBodyCell class="text-start text-wrap">
    {#if name}
      ┃
      <ShowMore bind:show_more {title} />
      {@html name}
    {:else}
      {#if part}
        <ShowMore bind:show_more {title} />
      {/if}
      <PlanName {plan} /> in
    {/if}
  </TableBodyCell>
  <TableBodyCell></TableBodyCell>
  {#if part}
    <PlanCell plan={plan.days} due={due.days} />
    <PlanCell plan={plan.rides} due={due.rides} />
    <PlanCell plan={plan.hours} due={due.hours} />
    <PlanCell plan={plan.km} due={due.km} />
    <PlanCell plan={plan.climb} due={due.climb} />
    <PlanCell plan={plan.descend} due={due.descend} />
    <PlanCell plan={plan.kJ} due={due.kJ} />
  {/if}

  <TableBodyCell>
    <div>
      <ChevronDownOutline class="cursor-pointer float-inline-right inline" />
      <Dropdown simple>
        {#if part}
          {@const plans = plan.id ? [plan.id] : []}
          <DropdownItem onclick={() => $actions.newService(part, plan)}>
            New Service for plan
          </DropdownItem>
          {#if plan.part != part.id}
            {@const att = part.attachments($attachments).at(0)}
            {#if att}
              <DropdownItem>
                <!-- <DropdownItem on:click={() => $actions.replacePart(att)}> -->
                Replace Part
              </DropdownItem>
            {/if}
          {/if}
        {/if}

        {#if !name && part}
          <DropdownDivider />
        {/if}

        {#if !name}
          <DropdownItem onclick={() => $actions.updatePlan(plan)}>
            Change ServicePlan
          </DropdownItem>
          <DropdownItem onclick={() => $actions.deletePlan(plan)}>
            Delete ServicePlan
          </DropdownItem>
        {/if}
      </Dropdown>
    </div>
  </TableBodyCell>
</TableBodyRow>
{#if part && show_more}
  {#each serviceList as service, i (service.id)}
    {@const successor = i > 0 ? serviceList[i - 1] : null}
    <ServiceRow depth={name ? 2 : 1} {part} {service} {successor} />
  {/each}
  <ServiceRow depth={name ? 1 : 0} {part} successor={serviceList.at(-1)} />
{/if}
