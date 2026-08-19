<script lang="ts">
  import { Badge, Tooltip } from "flowbite-svelte";

  let {
    service,
    pos,
  }: { service?: { due: number; plan: number }; pos?: string } = $props();

  function get_color(plan: number, due: number): any {
    if (due < 0) return "red";
    if (due < plan * 0.05) return "yellow";
    return "green";
  }

  let color = $derived(service ? get_color(service.plan, service.due) : null);
</script>

{#if color}
  <span class={pos}>
    <Badge {color} class="p-1.5 py-0.5">
      {Math.round(((service!.plan - service!.due) / service!.plan) * 100)}%
      <Tooltip>{service!.plan - service!.due}/{service!.plan}</Tooltip>
    </Badge>
  </span>
{/if}
