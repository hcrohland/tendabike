<script lang="ts">
  import { DropdownItem } from "@sveltestrap/sveltestrap";
  import ServiceRow from "../Service/ServiceRow.svelte";
  import { actions } from "../Widgets/Actions.svelte";
  import Menu from "../Widgets/Menu.svelte";
  import ShowHist from "../Widgets/ShowHist.svelte";
  import { attachments } from "../lib/attachment";
  import { parts } from "../lib/part";
  import { services } from "../lib/service";
  import { ServicePlan } from "../lib/serviceplan";
  import { usages } from "../lib/usage";
  import PlanCell from "./PlanCell.svelte";
  import PlanName from "./PlanName.svelte";

  export let plan: ServicePlan;
  export let name: string | null = null;

  let show_hist = false;

  $: part = plan.getpart($parts, $attachments);
  $: serviceList = plan.services(part, $services);
  $: due = plan.due(part, serviceList.at(0), $usages);
</script>

<tr>
  <td>
    {#if name}
      ┃
      <ShowHist bind:show_hist />
      {@html name}
    {:else}
      {#if part}
        <ShowHist bind:show_hist />
      {/if}
      <PlanName {plan} />
    {/if}
  </td>
  {#if !part}
    <td colspan="8" />
  {:else}
    <td class=""> in </td>
    <PlanCell plan={plan.days} due={due.days} />
    <PlanCell plan={plan.rides} due={due.rides} />
    <PlanCell plan={plan.hours} due={due.hours} />
    <PlanCell plan={plan.km} due={due.km} />
    <PlanCell plan={plan.climb} due={due.climb} />
    <PlanCell plan={plan.descend} due={due.descend} />
    <PlanCell plan={plan.kJ} due={due.kJ} />
  {/if}

  <td>
    <Menu>
      {#if !name}
        <DropdownItem on:click={() => $actions.updatePlan(plan)}>
          Change ServicePlan
        </DropdownItem>
        <DropdownItem on:click={() => $actions.deletePlan(plan)}>
          Delete ServicePlan
        </DropdownItem>
      {/if}

      {#if !name && part}
        <DropdownItem divider />
      {/if}

      {#if part}
        {#if serviceList.at(0) != undefined}
          <DropdownItem
            on:click={() => $actions.redoService(serviceList.at(0))}
          >
            Repeat last service
          </DropdownItem>
        {/if}
        {@const plans = plan.id ? [plan.id] : []}
        <DropdownItem on:click={() => $actions.newService(part, plans)}>
          New Service for plan
        </DropdownItem>
        {#if plan.part != part.id}
          {@const att = part.attachments($attachments).at(0)}
          {#if att}
            <DropdownItem on:click={() => $actions.replacePart(att)}>
              Replace Part
            </DropdownItem>
          {/if}
        {/if}
      {/if}
    </Menu>
  </td>
</tr>
{#if part && show_hist}
  {#each serviceList as service, i (service.id)}
    {@const successor = i > 0 ? serviceList[i - 1] : null}
    <ServiceRow depth={name ? 2 : 1} {part} {service} {successor} />
  {/each}
  <ServiceRow depth={name ? 1 : 0} {part} successor={serviceList.at(-1)} />
{/if}
