<script lang="ts">
  import { services, Service } from "../lib/service";
  import ShowMore from "../Widgets/ShowMore.svelte";
  import ServiceRow from "./ServiceRow.svelte";
  import { parts } from "../lib/part";

  interface Props {
    service: Service;
  }

  let { service }: Props = $props();

  let show_more = $state(false);

  let part = $derived($parts[service.part_id]);
  let successor = $derived(service.get_successor($services));
</script>

<div class="rounded-lg border border-border-subtle bg-surface-1 p-3">
  <ServiceRow {part} {service} {successor}>
    <ShowMore bind:show_more title="history" />
  </ServiceRow>
  {#if show_more}
    <div class="rounded-lg border border-border-subtle bg-surface-1 p-3">
      <div class="flex flex-col gap-2 mt-2">
        {#each service.history(1, $services) as s (s.service?.id + "-" + s.successor?.id)}
          <ServiceRow {part} {...s} />
        {/each}
      </div>
    </div>
  {/if}
</div>
