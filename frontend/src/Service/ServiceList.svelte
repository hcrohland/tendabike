<script lang="ts">
  import { Button } from "@sveltestrap/sveltestrap";
  import { services, filterValues, by } from "../lib/store";
  import NewService from "./NewService.svelte";
  import { Part } from "../lib/types";
  import ServiceHist from "./ServiceHist.svelte";
  import ServiceHeader from "./ServiceHeader.svelte";

  export let part: Part;

  let newService: (p: Part) => void;

  $: servs = filterValues(
    $services,
    (s) => s.part_id == part.id && s.successor == undefined,
  ).sort(by("time"));
</script>

<div class="table-responsive">
  <table class="table">
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
  </table>
</div>

<NewService bind:newService />
