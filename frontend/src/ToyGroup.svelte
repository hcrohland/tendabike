<script lang="ts">
  import MainCard from "./Part/MainCard.svelte";
  import { filterValues, by } from "./lib/mapable";
  import { category } from "./lib/types";
  import { parts } from "./lib/part";
  import { activities } from "./lib/activity";
  import ShowMore from "./Widgets/ShowMore.svelte";
  import { shop } from "./lib/shop";
  import * as m from "../paraglide/messages";

  let show_more: boolean = $state(false);

  let gears = $derived(
    filterValues(
      $parts,
      (p) =>
        ($shop ? p.shop == $shop.id : true) &&
        p.what == $category.id &&
        !p.disposed_at,
    ).sort(by("last_used")),
  );
  let bin = $derived(
    filterValues(
      $parts,
      (p) =>
        ($shop ? p.shop == $shop.id : true) &&
        p.what == $category.id &&
        p.disposed_at != undefined,
    ).sort(by("last_used")),
  );
</script>

{#if $category}
  <div class="grid grid-cols-1 sm:grid-cols-2 gap-4">
    {#each gears as part (part.id)}
      <MainCard {part} />
    {:else}
      {#if $category.activities($activities).length == 0}
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
          <MainCard {part} />
        {/each}
      </div>
    {/if}
  {/if}
{:else}
  {m.toygroup_category_not_found()}
{/if}
