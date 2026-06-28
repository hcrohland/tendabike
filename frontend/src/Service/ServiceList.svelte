<script lang="ts">
  import { filterValues, by } from "../lib/mapable";
  import { Part } from "../lib/part";
  import ServiceHist from "./ServiceHist.svelte";
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

<div class="flex flex-col gap-3">
  {#each servs as service (service.id)}
    <ServiceHist {service} />
  {/each}
</div>
