<script lang="ts">
  import { Column, Table } from "@sveltestrap/sveltestrap";
  import { plans } from "../lib/serviceplan";
  import { fmtNumber } from "../lib/store";
  import PlanName from "../ServicePlan/PlanName.svelte";

  $: list = Object.values($plans);
  function fmt(x: number | null | undefined) {
    return x == null ? "-" : fmtNumber(x);
  }
</script>

<div class="table-responsive">
  <Table rows={list} let:row responsive hover>
    <Column header="Name">
      <PlanName plan={row} full />
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
  </Table>
</div>
