<script lang="ts">
  import { attachments } from "../lib/attachment";
  import { filterValues } from "../lib/mapable";
  import type { Part } from "../lib/part";
  import { plans, plansForAssembly, planCmp } from "../lib/serviceplan";
  import { category, types } from "../lib/types";
  import PlanBlock from "./PlanBlock.svelte";

  interface Props {
    part?: Part | undefined;
    children?: import("svelte").Snippet;
  }

  let { part: gear, children }: Props = $props();
  let planlist = $derived(
    (gear
      ? plansForAssembly(gear, $plans, $attachments)
      : filterValues($plans, (p) => types[p.what].main == $category.main)
    ).sort(planCmp),
  );
</script>

<div class="flex flex-col gap-3">
  {#if children}
    <div class="flex justify-end">
      {@render children?.()}
    </div>
  {/if}
  {#each planlist as plan (plan.id)}
    <PlanBlock {plan} />
  {/each}
</div>
