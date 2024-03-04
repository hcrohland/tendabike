<script lang="ts">
  import { Button, Table } from "@sveltestrap/sveltestrap";
  import { filterValues, by } from "../lib/mapable";
  import { Part } from "../Part/part";
  import { plans, plans_for_gear } from "./serviceplan";
  import { services } from "../Service/service";
  import PlanHeader from "./PlanHeader.svelte";
  import PlanRow from "./PlanRow.svelte";
  import NewPlan from "./NewPlan.svelte";
  import NewService from "../Service/NewService.svelte";
  import { attachments } from "../Attachment/attachment";

  export let part: Part;

  let newPlan: (p: Part) => void;
  let newService: (p: Part) => void;
</script>

<Table responsive hover>
  <thead>
    <PlanHeader>
      Service Plans &nbsp;&nbsp;
      <Button size="sm" color="light" on:click={() => newPlan(part)}>
        add
      </Button>
    </PlanHeader>
  </thead>
  <tbody>
    {#each plans_for_gear(part.id, $plans, $attachments) as plan (plan.id)}
      <PlanRow {plan} />
    {/each}
  </tbody>
</Table>

<NewPlan bind:newPlan />
<NewService bind:newService />
