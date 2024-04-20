<script lang="ts">
  import { DropdownItem } from "@sveltestrap/sveltestrap";
  import DeletePlan from "./DeletePlan.svelte";
  import Menu from "../Widgets/Menu.svelte";
  import PlanHook from "./PlanHook.svelte";
  import PlanCell from "./PlanCell.svelte";
  import { plans, ServicePlan } from "../lib/serviceplan";
  import { Attachment, attachments } from "../lib/attachment";
  import { Part, parts } from "../lib/part";
  import { Service, services } from "../lib/service";
  import { usages } from "../lib/usage";
  import ShowHist from "../Widgets/ShowHist.svelte";
  import UpdatePlan from "./UpdatePlan.svelte";
  import ServiceRow from "../Service/ServiceRow.svelte";
  import NewService from "../Service/NewService.svelte";
  import RedoService from "../Service/RedoService.svelte";
  import ReplacePart from "../Attachment/ReplacePart.svelte";

  export let plan: ServicePlan;

  let updatePlan: (p: ServicePlan) => void;
  let deletePlan: (p: ServicePlan) => void;
  let newService: (part: Part, plans?: string[]) => void;
  let replacePart: (a: Attachment) => void;
  let redoService: (s: Service | undefined) => void;

  let show_hist = false;

  let no_template = plan.id && $plans[plan.id].part;

  $: part = plan.getpart($parts, $attachments);
  $: serviceList = plan.services(part, $services);

  $: due = plan.due(part, serviceList.at(0), $usages);
</script>

<tr>
  <td>
    <div>
      <span id={"name" + plan.id}>
        <ShowHist bind:show_hist />
        {plan.name}
        <PlanHook {plan} />
        {#if plan.hook && no_template}
          for {@html $parts[plan.part].partLink()}
        {/if}
      </span>
    </div>
  </td>
  <td class=""> after </td>
  <PlanCell plan={plan.days} due={due.days} />
  <PlanCell plan={plan.rides} due={due.rides} />
  <PlanCell plan={plan.hours} due={due.hours} />
  <PlanCell plan={plan.km} due={due.km} />
  <PlanCell plan={plan.climb} due={due.climb} />
  <PlanCell plan={plan.descend} due={due.descend} />

  <td>
    <Menu>
      {#if !(part.isGear() && plan.hook)}
        {#if serviceList.at(0) != undefined}
          <DropdownItem on:click={() => redoService(serviceList.at(0))}>
            Repeat last service
          </DropdownItem>
        {/if}
        {@const plans = plan.id ? [plan.id] : []}
        <DropdownItem on:click={() => newService(part, plans)}>
          New Service for plan
        </DropdownItem>
        {#if plan.part != part.id}
          {@const att = part.attachments($attachments).at(0)}
          {#if att}
            <DropdownItem on:click={() => replacePart(att)}>
              Replace Part
            </DropdownItem>
          {/if}
        {/if}
        <DropdownItem divider />
      {/if}
      <DropdownItem on:click={() => updatePlan(plan)}>
        Change ServicePlan
      </DropdownItem>
      <DropdownItem on:click={() => deletePlan(plan)}>
        Delete ServicePlan
      </DropdownItem>
    </Menu>
  </td>
</tr>
{#if show_hist}
  {#each serviceList as service, i (service.id)}
    {@const successor = i > 0 ? serviceList[i - 1] : null}
    <ServiceRow depth={1} {part} {service} {successor} />
  {/each}
  <ServiceRow {part} successor={serviceList.at(-1)} />
{/if}
<UpdatePlan bind:updatePlan />
<DeletePlan bind:deletePlan />
<NewService bind:newService />
<RedoService bind:redoService />
<ReplacePart bind:replacePart />
