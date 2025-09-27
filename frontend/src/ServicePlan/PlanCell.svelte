<script lang="ts">
  import { Tooltip } from "flowbite-svelte";
  import { fmtNumber } from "../lib/store";

  interface Props {
    plan: number | null;
    due: number | null;
    fmt?: any;
  }

  let { plan, due, fmt = fmtNumber }: Props = $props();
  let id = "alert-" + plan + "-" + due;

  function get_class(plan: number, due: number) {
    if (due < 0) return "rounded p-1 bg-red-600 text-white";
    if (due < plan * 0.05) return "rounded p-1 text-gray-900 bg-yellow-200";
    return "";
  }
</script>

<td class="text-end">
  {#if plan != null && due != null}
    <span class={get_class(plan, due)}>
      {fmt(due)}
    </span>
    <Tooltip>
      {fmt(plan - due)} / {fmt(plan)}
    </Tooltip>
  {:else}
    -
  {/if}
</td>
