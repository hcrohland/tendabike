<script lang="ts">
  import { services, Service } from "../lib/service";
  import ShowMore from "../Widgets/ShowMore.svelte";
  import ServiceRow from "./ServiceRow.svelte";
  import { parts } from "../lib/part";

  export let service: Service;
  export let depth = 0;

  let show_more = false;

  $: part = $parts[service.part_id];
  $: successor = service.get_successor($services);
</script>

<ServiceRow {part} {service} {successor} {depth}>
  <ShowMore bind:show_more title="history" />
</ServiceRow>
{#if show_more}
  {#each service.history(1, $services) as s (s.service?.id + "-" + s.successor?.id)}
    <ServiceRow {part} {...s} />
  {/each}
{/if}
