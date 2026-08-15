<script lang="ts">
  import { attachments } from "../lib/attachment";
  import { filterValues } from "../lib/mapable";
  import type { Part } from "../lib/part";
  import {
    plans,
    plans_for_part_and_subtypes,
    ServicePlan,
  } from "../lib/serviceplan";
  import { category, types } from "../lib/types";
  import PlanBlock from "./PlanBlock.svelte";

  interface Props {
    part?: Part | undefined;
    children?: import("svelte").Snippet;
  }

  let { part: gear, children }: Props = $props();
  let planlist = $derived(
    gear
      ? plans_for_part_and_subtypes($attachments, $plans, gear)
      : filterValues($plans, (p) => types[p.what].main == $category.main),
  );
  function cmp(p: ServicePlan, q: ServicePlan) {
    let res;
    if (p.what != q.what) {
      res = p.what < q.what;
    } else if (p.hook != q.hook) {
      res = p.what < q.what;
    } else if (p.part != q.part) {
      res = p.part! < q.part!;
    } else {
      res = p.id! < q.id!;
    }
    return res ? -1 : 1;
  }
</script>

<div class="flex flex-col gap-3">
  {#if children}
    <div class="flex justify-end">
      {@render children?.()}
    </div>
  {/if}
  {#each planlist.sort(cmp) as plan (plan.id)}
    <PlanBlock {plan} />
  {/each}
</div>
