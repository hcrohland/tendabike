<script lang="ts">
  import { DropdownItem } from "@sveltestrap/sveltestrap";
  import DeletePlan from "./DeletePlan.svelte";
  import Menu from "../Widgets/Menu.svelte";
  import PlanHook from "./PlanHook.svelte";
  import PlanCell from "./PlanCell.svelte";
  import { ServicePlan } from "./serviceplan";
  import { attachments } from "../Attachment/attachment";
  import { Part, parts } from "../Part/part";
  import { services } from "../Service/service";
  import { usages } from "../Usage/usage";
  import ShowHist from "../Widgets/ShowHist.svelte";
  import UpdatePlan from "./UpdatePlan.svelte";
  import ServiceRow from "../Service/ServiceRow.svelte";
  import NewService from "../Service/NewService.svelte";

  export let plan: ServicePlan;

  let updatePlan: (p: ServicePlan) => void;
  let deletePlan: (p: ServicePlan) => void;
  let newService: (part: Part, plans?: string[]) => void;

  let show_hist = false;

  $: part = plan.getpart($parts, $attachments);
  $: serviceList = plan.services(part, $services);

  $: due = plan.due(part, serviceList.at(0), $usages);
</script>

<tr>
  <td>
    <div>
      <span id={"name" + plan.id}>
        {plan.name}
        <PlanHook {plan} />
        <ShowHist bind:show_hist />
      </span>
    </div>
  </td>
  <td class="text-end"> in </td>
  <PlanCell plan={plan.days} due={due.days} />
  <PlanCell plan={plan.rides} due={due.rides} />
  <PlanCell plan={plan.time} due={due.time} />
  <PlanCell plan={plan.distance} due={due.distance} />
  <PlanCell plan={plan.climb} due={due.climb} />
  <PlanCell plan={plan.descend} due={due.descend} />

  <td>
    <Menu>
      <DropdownItem on:click={() => newService(part, plan.id ? [plan.id] : [])}>
        Log Service for plan
      </DropdownItem>
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