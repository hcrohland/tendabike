<script lang="ts">
  import { Table } from "flowbite-svelte";
  import { filterValues, by } from "../lib/mapable";
  import { Part } from "../lib/part";
  import ServiceHist from "./ServiceHist.svelte";
  import ServiceHeader from "./ServiceHeader.svelte";
  import { services } from "../lib/service";

  export let part: Part;

  $: servs = filterValues(
    $services,
    (s) => s.part_id == part.id && s.successor == undefined,
  ).sort(by("time"));
</script>

<div class="table-responsive">
  <Table responsive hover>
    <thead>
      <ServiceHeader />
    </thead>
    <tbody>
      {#each servs as service (service.id)}
        <ServiceHist {service} />
      {/each}
    </tbody>
  </Table>
</div>
