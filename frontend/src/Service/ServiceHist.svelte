<script lang="ts">
  import { services, usages, parts } from "../lib/store";
  import { Service } from "./service";
  import ShowHist from "../Widgets/ShowHist.svelte";
  import ServiceRow from "./ServiceRow.svelte";

  export let service: Service;

  let show_hist = false;
</script>

<ServiceRow {...service.get_row($parts, $usages, $services)}>
  <ShowHist bind:show_hist />
</ServiceRow>
{#if show_hist}
  {#each service.predecessors($services) as s (s.id)}
    <ServiceRow {...s.get_row($parts, $usages, $services)} />
  {/each}
{/if}
