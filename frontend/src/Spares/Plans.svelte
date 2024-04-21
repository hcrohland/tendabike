<script lang="ts">
  import { Column, DropdownItem, Table } from "@sveltestrap/sveltestrap";
  import { ServicePlan, plans } from "../lib/serviceplan";
  import { fmtNumber } from "../lib/store";
  import PlanName from "../ServicePlan/PlanName.svelte";
  import UpdatePlan from "../ServicePlan/UpdatePlan.svelte";
  import DeletePlan from "../ServicePlan/DeletePlan.svelte";
  import Menu from "../Widgets/Menu.svelte";

  let updatePlan: (p: ServicePlan | undefined) => void;
  let deletePlan: (p: ServicePlan | undefined) => void;

  function fmt(x: number | null | undefined) {
    return x == null ? "-" : fmtNumber(x);
  }
</script>

<Table rows={Object.values($plans)} let:row responsive hover>
  <Column header="Name">
    <PlanName plan={row} />
  </Column>
  <Column header="Days">
    {fmt(row?.days)}
  </Column>
  <Column header="Rides">
    {fmt(row?.rides)}
  </Column>
  <Column header="km">
    {fmt(row?.km)}
  </Column>
  <Column header="Climb">
    {fmt(row?.climb)}
  </Column>
  <Column header="Descend">
    {fmt(row?.descend)}
  </Column>
  <Column>
    <Menu>
      <DropdownItem on:click={() => updatePlan(row)}>
        Change ServicePlan
      </DropdownItem>
      <DropdownItem on:click={() => deletePlan(row)}>
        Delete ServicePlan
      </DropdownItem>
    </Menu>
  </Column>
</Table>

<UpdatePlan bind:updatePlan />
<DeletePlan bind:deletePlan />
