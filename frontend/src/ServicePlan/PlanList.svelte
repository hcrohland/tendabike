<script lang="ts">
  import { Button } from "@sveltestrap/sveltestrap";
  import { filterValues, by } from "../lib/mapable";
  import { Part } from "../Part/part";
  import { plans, plans_for_part } from "./serviceplan";
  import { services } from "../Service/service";
  import PlanHeader from "./PlanHeader.svelte";
  import PlanRow from "./PlanRow.svelte";
  import NewPlan from "./NewPlan.svelte";
  import ServiceHist from "../Service/ServiceHist.svelte";
  import NewService from "../Service/NewService.svelte";
  import { attachments } from "../Attachment/attachment";

  export let part: Part;

  let newPlan: (p: Part) => void;
  let newService: (p: Part) => void;

  $: servs = filterValues(
    $services,
    (s) => s.part_id == part.id && s.successor == undefined && s.plan == null,
  ).sort(by("time"));
</script>

<div class="table-responsive">
  <table class="table">
    <thead>
      <PlanHeader>
        Service Plans &nbsp;&nbsp;
        <Button size="sm" color="light" on:click={() => newPlan(part)}>
          add
        </Button>
      </PlanHeader>
    </thead>
    <tbody>
      {#each plans_for_part(part.id, $plans, $attachments) as plan (plan.id)}
        <PlanRow {plan} />
      {/each}
      <thead>
        <th colspan="20">
          More Service Logs &nbsp;&nbsp;
          <Button size="sm" color="light" on:click={() => newService(part)}>
            add
          </Button>
        </th>
      </thead>
      {#each servs as service (service.id)}
        <ServiceHist {service} />
      {/each}
    </tbody>
  </table>
</div>

<NewPlan bind:newPlan />
<NewService bind:newService />
