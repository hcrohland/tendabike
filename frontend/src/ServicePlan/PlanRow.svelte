<script lang="ts">
  import ServiceRow from "../Service/ServiceRow.svelte";
  import { attachments } from "../lib/attachment";
  import { Part, parts } from "../lib/part";
  import { services } from "../lib/service";
  import {
    partForPlanGear,
    duesForPlans,
    ServicePlan,
  } from "../lib/serviceplan";
  import { usages } from "../lib/usage";
  import ShowMore from "../Widgets/ShowMore.svelte";
  import * as m from "../../paraglide/messages";
  import Menu from "../Widgets/Menu.svelte";
  import { DropdownItem } from "flowbite-svelte";
  import { getActions } from "../Widgets/Actions.svelte";
  import XsButton from "../Widgets/XsButton.svelte";

  interface Props {
    plan: ServicePlan;
    gear?: Part | undefined;
  }

  let { plan, gear = undefined }: Props = $props();

  let show_more = $state(false);

  let part = $derived(
    partForPlanGear(plan, gear?.id, parts, attachments),
  ) as Part;
  let [ActiveService, ...serviceList] = $derived(plan.services(part, services));
  let due_list = $derived(duesForPlans(part, [plan], services, usages));
</script>

{#if part}
  <div class="rounded-lg border border-border-subtle bg-surface-1 p-3">
    <!-- Header -->
    <div class="flex items-center justify-between gap-2">
      <div class="flex items-center gap-2 min-w-0">
        <span class="text-sm shrink-0">
          {#if gear?.id != part.id && plan.part != gear?.id}
            {@html gear!.partLink()}:
          {/if}
          {@html part.partLink()}
        </span>

        {#if serviceList.length > 0}
          <ShowMore bind:show_more title={m.planrow_service_history()} />
        {/if}
      </div>
      {#if plan.what == part.what}
        <Menu>
          <DropdownItem onclick={() => getActions()!.newService(part, plan)}>
            {m.planmenu_new_service()}
          </DropdownItem>
          {#if plan.part != part.id}
            {@const att = part.attachments(attachments).at(0)}
            {#if att}
              <DropdownItem onclick={() => getActions()!.replacePart(att)}>
                {m.action_replace()}
              </DropdownItem>
            {/if}
          {/if}
        </Menu>
      {:else}
        <XsButton onclick={() => getActions()!.installPart(part)}>
          {m.action_new()}
        </XsButton>
      {/if}
    </div>

    {#if plan.what == part.what}
      <!-- Service history -->
      <div class="flex flex-col gap-2 mt-3">
        <ServiceRow {part} service={ActiveService} {due_list} light />
        {#if show_more}
          {#each serviceList as service, i (service.id)}
            {@const successor = i > 0 ? serviceList[i - 1] : ActiveService}
            <ServiceRow {part} {service} {successor} light />
          {/each}
          <ServiceRow {part} successor={serviceList.at(-1)} light />
        {/if}
      </div>
    {/if}
  </div>
{/if}
