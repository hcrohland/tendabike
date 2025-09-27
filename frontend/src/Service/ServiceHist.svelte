<script lang="ts">
  import { services, Service } from "../lib/service";
  import ShowMore from "../Widgets/ShowMore.svelte";
  import ServiceRow from "./ServiceRow.svelte";
  import { parts } from "../lib/part";

  interface Props {
    service: Service;
    depth?: number;
  }

  let { service, depth = 0 }: Props = $props();

  let show_more = $state(false);

  let part = $derived($parts[service.part_id]);
  let successor = $derived(service.get_successor($services));
</script>

<ServiceRow {part} {service} {successor} {depth}>
  <ShowMore bind:show_more title="history" />
</ServiceRow>
{#if show_more}
  {#each service.history(1, $services) as s (s.service?.id + "-" + s.successor?.id)}
    <ServiceRow {part} {...s} />
  {/each}
{/if}
