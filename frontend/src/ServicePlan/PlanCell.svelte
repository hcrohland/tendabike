<script lang="ts">
  import { Tooltip } from "@sveltestrap/sveltestrap";
  import { fmtNumber } from "../lib/store";

  export let plan: number | null;
  export let due: number | null;
  export let fmt = fmtNumber;
  let id = "alert-" + plan + "-" + due;

  function get_class(plan: number, due: number) {
    if (due < 0) return "rounded p-1 bg-danger text-white";
    if (due < plan * 0.05) return "rounded p-1 bg-warning";
    return "";
  }
</script>

<td class="text-end">
  {#if plan != null && due != null}
    <span {id} class={get_class(plan, due)}>
      {fmt(due)}
    </span>
    <Tooltip target={id}>
      {fmt(plan - due)} / {fmt(plan)}
    </Tooltip>
  {:else}
    -
  {/if}
</td>
