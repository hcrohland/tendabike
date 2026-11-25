<script lang="ts">
  import ServiceRow from "../Service/ServiceRow.svelte";
  import { Part } from "../lib/part";
  import {
    alertForPlan,
    partForPlanGear,
    duesForPlans,
    ServicePlan,
  } from "../lib/serviceplan";
  import { bg, border } from "../Widgets/tone.js";
  import ShowMore from "../Widgets/ShowMore.svelte";
  import * as m from "../../paraglide/messages";
  import Menu from "../Widgets/Menu.svelte";
  import { Badge, DropdownItem } from "flowbite-svelte";
  import { newService } from "../Service/ServiceActions.svelte";
  import { snoozePlan } from "./SnoozePlan.svelte";
  import { replacePart } from "../Attachment/ReplacePart.svelte";
  import { installPart } from "../Attachment/InstallPart.svelte";
  import XsButton from "../Widgets/XsButton.svelte";

  interface Props {
    plan: ServicePlan;
    gear?: Part | undefined;
  }

  let { plan, gear = undefined }: Props = $props();

  let show_more = $state(false);

  let part = $derived(partForPlanGear(plan, gear?.id)) as Part;
  let [ActiveService, ...serviceList] = $derived(plan.services(part));
  let due_list = $derived(duesForPlans(part, [plan]));
  // The (part, plan) is snoozed when the plan carries an accumulated
  // fraction for it; the action is offered only while it is in the warn band
  // or overdue, never when it is ok (no pre-snooze).
  let snoozed = $derived(part.id != null && (plan.snoozes[part.id] ?? 0) > 0);
  let severity = $derived(alertForPlan(plan, part, ActiveService));
</script>

{#if part}
  <div class={"rounded-lg border " + border() + " " + bg("primary") + " p-3"}>
    <!-- Header -->
    <div class="flex items-center justify-between gap-2">
      <div class="flex items-center gap-2 min-w-0">
        <span class="text-sm shrink-0">
          {#if gear?.id != part.id && plan.part != gear?.id}
            {@html gear!.partLink()}:
          {/if}
          {@html part.partLink()}
        </span>

        {#if snoozed}
          <Badge color="violet" class="shrink-0">
            {m.planrow_snoozed()}
          </Badge>
        {/if}

        {#if serviceList.length > 0}
          <ShowMore bind:show_more title={m.planrow_service_history()} />
        {/if}
      </div>
      {#if plan.what == part.what}
        <Menu>
          <DropdownItem onclick={() => newService(part, plan)}>
            {m.planmenu_new_service()}
          </DropdownItem>
          {#if severity == "warn" || severity == "alert"}
            <DropdownItem onclick={() => snoozePlan(plan, part)}>
              {m.planmenu_snooze()}
            </DropdownItem>
          {/if}
          {#if plan.part != part.id}
            {@const att = part.attachments().at(0)}
            {#if att}
              <DropdownItem onclick={() => replacePart(att)}>
                {m.action_replace()}
              </DropdownItem>
            {/if}
          {/if}
        </Menu>
      {:else}
        <XsButton onclick={() => installPart(part)}>
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
