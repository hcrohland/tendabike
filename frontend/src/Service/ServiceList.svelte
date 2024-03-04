<script lang="ts">
  import { Button, Table } from "@sveltestrap/sveltestrap";
  import { filterValues, by } from "../lib/mapable";
  import NewService from "./NewService.svelte";
  import { Part } from "../Part/part";
  import ServiceHist from "./ServiceHist.svelte";
  import ServiceHeader from "./ServiceHeader.svelte";
  import { services } from "./service";

  export let part: Part;

  let newService: (p: Part) => void;

  $: servs = filterValues(
    $services,
    (s) => s.part_id == part.id && s.successor == undefined,
  ).sort(by("time"));
</script>

<div class="table-responsive">
  <Table responsive hover>
    <thead>
      <ServiceHeader>
        Service Log &nbsp;&nbsp;
        <Button size="sm" color="light" on:click={() => newService(part)}>
          add
        </Button>
      </ServiceHeader>
    </thead>
    <tbody>
      {#each servs as service (service.id)}
        <ServiceHist {service} />
      {/each}
    </tbody>
  </Table>
</div>

<NewService bind:newService />
