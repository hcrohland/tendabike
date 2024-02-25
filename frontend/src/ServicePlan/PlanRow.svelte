<script lang="ts">
  import { DropdownItem } from "@sveltestrap/sveltestrap";
  import DeletePlan from "./DeletePlan.svelte";
  // import UpdatePlan from "./UpdatePlan.svelte";
  // import RedoPlan from "./RedoPlan.svelte";
  import Menu from "../Widgets/Menu.svelte";
  import ServiceHist from "../Service/ServiceHist.svelte";
  import PlanHook from "./PlanHook.svelte";
  import PlanCell from "./PlanCell.svelte";
  import { ServicePlan } from "./serviceplan";
  import { attachments } from "../Attachment/attachment";
  import { parts } from "../Part/part";
  import { services } from "../Service/service";
  import { usages } from "../Usage/usage";
  import { fmtDate, fmtSeconds, get_days } from "../lib/store";
  import Usage from "../Usage/Usage.svelte";
  import ShowHist from "../Widgets/ShowHist.svelte";
  import UpdatePlan from "./UpdatePlan.svelte";

  export let plan: ServicePlan;

  let updatePlan: (p: ServicePlan) => void;
  // let fullfillPlan: (p: ServicePlan) => void;
  let deletePlan: (p: ServicePlan) => void;

  let show_hist = false;

  $: status = plan.status($services, $parts, $attachments, $usages);
  $: due = plan.due(status.time, status.usage);
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
  <PlanCell plan={plan.time} due={due.time} fmt={fmtSeconds} />
  <PlanCell plan={plan.distance} due={due.distance} />
  <PlanCell plan={plan.climb} due={due.climb} />
  <PlanCell plan={plan.descend} due={due.descend} />

  <td>
    <Menu>
      <DropdownItem on:click={() => updatePlan(plan)}>
        Change ServicePlan
      </DropdownItem>
      <!-- <DropdownItem on:click={() => fullfillPlan(plan)}>
          Repeat ServicePlan
        </DropdownItem> -->
      <DropdownItem on:click={() => deletePlan(plan)}>
        Delete ServicePlan
      </DropdownItem>
    </Menu>
  </td>
</tr>
{#if show_hist}
  {#if status.service}
    <ServiceHist service={status.service} show_all depth={1} />
  {:else}
    {@const p = status.part}
    {@const now = new Date()}
    <tr>
      <td> ┗━</td>
      <td>{fmtDate(p.purchase)}</td>
      <td class="text-end">{get_days(p.purchase)}</td>
      <Usage usage={$usages[p.usage]} />
    </tr>
  {/if}
{/if}
<UpdatePlan bind:updatePlan />
<DeletePlan bind:deletePlan />
<!-- <RedoPlan bind:fullfillPlan /> -->
