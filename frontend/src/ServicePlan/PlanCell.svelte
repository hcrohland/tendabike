<script lang="ts">
  import { Badge, Tooltip } from "@sveltestrap/sveltestrap";
  import { fmtNumber } from "../lib/store";

  export let plan: number | null;
  export let due: number | null;
  export let fmt = fmtNumber;
  let id = "alert-" + plan + "-" + due;
</script>

<td class="text-end">
  {#if plan != null && due != null}
    <span {id}>
      {#if due <= 0}
        <Badge color="danger">!</Badge>
      {:else if due < plan * 0.05}
        <Badge color="warning">!</Badge>
      {/if}
      <slot>{fmt(due)}</slot>
    </span>
    <Tooltip target={id}>
      {plan - due} / {plan}
    </Tooltip>
  {:else}
    -
  {/if}
</td>
