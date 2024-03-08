<script lang="ts">
  import { usages } from "../Usage/usage";
  import { services, Service } from "./service";
  import ShowHist from "../Widgets/ShowHist.svelte";
  import ServiceRow from "./ServiceRow.svelte";
  import { parts } from "../lib/part";

  export let service: Service;
  export let depth = 0;

  export let show_all = false;

  let show_hist = false;

  $: part = $parts[service.part_id];
  $: successor = service.get_successor($services);
</script>

<ServiceRow {part} {service} {successor} {depth}>
  {#if !show_all}
    <ShowHist bind:show_hist />
  {/if}
</ServiceRow>
{#if show_hist || show_all}
  {#each service.history(1, $services) as s (s.service?.id + "-" + s.successor?.id)}
    <ServiceRow {part} {...s} />
  {/each}
{/if}
