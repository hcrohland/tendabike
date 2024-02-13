<script lang="ts">
  import { usages } from "../Usage/usage";
  import { services, Service } from "./service";
  import ShowHist from "../Widgets/ShowHist.svelte";
  import ServiceRow from "./ServiceRow.svelte";
  import { parts } from "../Part/part";

  export let service: Service;
  export let depth = 0;

  export let show_all = false;

  let show_hist = false;
</script>

<ServiceRow {...service.get_row(depth, $parts, $usages, $services)}>
  {#if !show_all}
    <ShowHist bind:show_hist />
  {/if}
</ServiceRow>
{#if show_hist || show_all}
  {#each service.history(1, $parts, $usages, $services) as s (s.service.id)}
    <ServiceRow {...s} />
  {/each}
{/if}
