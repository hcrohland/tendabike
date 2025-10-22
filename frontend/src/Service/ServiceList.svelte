<script lang="ts">
  import { Table, TableBody, TableHead } from "flowbite-svelte";
  import { filterValues, by } from "../lib/mapable";
  import { Part } from "../lib/part";
  import ServiceHist from "./ServiceHist.svelte";
  import ServiceHeader from "./ServiceHeader.svelte";
  import { services } from "../lib/service";

  interface Props {
    part: Part;
  }

  let { part }: Props = $props();

  let servs = $derived(
    filterValues(
      $services,
      (s) => s.part_id == part.id && s.successor == undefined,
    ).sort(by("time")),
  );
</script>

<Table hoverable striped>
  <TableHead>
    <ServiceHeader />
  </TableHead>
  <TableBody>
    {#each servs as service (service.id)}
      <ServiceHist {service} />
    {/each}
  </TableBody>
</Table>
