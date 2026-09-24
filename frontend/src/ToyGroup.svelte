<script lang="ts">
  import { by } from "./lib/mapable";
  import { stateValues } from "./lib/mapable.svelte";
  import { category } from "./lib/types";
  import { parts } from "./lib/part";
  import { activities } from "./lib/activity";
  import ShowMore from "./Widgets/ShowMore.svelte";
  import { getShop } from "./lib/shop";
  import * as m from "../paraglide/messages";
  import GearCard from "./Part/GearCard.svelte";
  import PlanBadge from "./ServicePlan/PlanBadge.svelte";

  let show_more: boolean = $state(false);

  let gears = $derived(
    stateValues(parts)
      .filter(
        (p) =>
          (getShop() ? p.shop == getShop()!.id : true) &&
          p.what == $category.id &&
          !p.disposed_at,
      )
      .sort(by("last_used")),
  );
  let bin = $derived(
    stateValues(parts)
      .filter(
        (p) =>
          (getShop() ? p.shop == getShop()!.id : true) &&
          p.what == $category.id &&
          p.disposed_at != undefined,
      )
      .sort(by("last_used")),
  );
</script>

{#if $category}
  <div class="grid grid-cols-1 sm:grid-cols-2 gap-4">
    {#each gears as part (part.id)}
      <GearCard {part} summary gridclass="grid-cols-3">
        <PlanBadge {part} />
      </GearCard>
    {:else}
      {#if $category.activities(activities).length == 0}
        {m.toygroup_none_found({ category: $category.name })}
      {:else}
        {m.toygroup_none_assigned({ category: $category.name })}
      {/if}
    {/each}
  </div>

  {#if bin.length > 0}
    <div class="p-4">
      <ShowMore bind:show_more title={m.sparetype_disposed()} />
    </div>
    {#if show_more}
      <div class="grid grid-cols-1 sm:grid-cols-2 gap-4">
        {#each bin as part (part.id)}
          <GearCard {part} summary gridclass="grid-cols-3">
            <PlanBadge {part} />
          </GearCard>
        {/each}
      </div>
    {/if}
  {/if}
{:else}
  {m.toygroup_category_not_found()}
{/if}
