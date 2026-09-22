<script lang="ts">
  import { Badge, Tooltip } from "flowbite-svelte";
  import type { Due } from "../lib/serviceplan";

  // The badge renders only: verdict → colour, remaining/limit → used
  // percentage. The severity rule lives in the plan module (issue #345).
  const verdict_color = {
    alert: "red",
    warn: "yellow",
    ok: "green",
  } as const;

  let { due, pos }: { due?: Due; pos?: string } = $props();

  let color = $derived(due ? verdict_color[due.severity] : null);
</script>

{#if color}
  <span class={pos} data-testid="badge-pos">
    <Badge {color} class="p-1.5 py-0.5">
      {Math.round(((due!.plan - due!.due) / due!.plan) * 100)}%
      <Tooltip>{due!.plan - due!.due}/{due!.plan}</Tooltip>
    </Badge>
  </span>
{/if}
