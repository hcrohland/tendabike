<script lang="ts">
  import { ServicePlan } from "../lib/serviceplan";
  import PlanBlock from "./PlanBlock.svelte";

  interface Props {
    planlist: ServicePlan[];
    children?: import("svelte").Snippet;
  }

  let { planlist, children }: Props = $props();

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
