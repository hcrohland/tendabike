<script lang="ts">
  import { usages } from "../Usage/usage";
  import { services, Service } from "./service";
  import ShowHist from "../Widgets/ShowHist.svelte";
  import ServiceRow from "./ServiceRow.svelte";
  import { parts } from "../Part/part";

  export let service: Service;

  let show_hist = false;
</script>

<ServiceRow {...service.get_row(0, $parts, $usages, $services)}>
  <ShowHist bind:show_hist />
</ServiceRow>
{#if show_hist}
  {#each service.history(0, $parts, $usages, $services) as s (s.service.id)}
    <ServiceRow {...s} />
  {/each}
{/if}
