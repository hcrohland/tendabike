<script lang="ts">
  import { Badge, Tooltip } from "flowbite-svelte";

  export let service: { due: number; plan: number } | undefined = undefined;
  export let pos = "";

  function get_color(plan: number, due: number): any {
    if (due < 0) return "red";
    if (due < plan * 0.05) return "yellow";
    return "green";
  }

  $: color = service ? get_color(service.plan, service.due) : null;
</script>

{#if color}
  <Badge {color} class={pos}>
    {service!.due}
    <Tooltip>{service!.plan - service!.due}/{service!.plan}</Tooltip>
  </Badge>
{/if}
