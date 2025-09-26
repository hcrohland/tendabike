<script lang="ts">
  import MainCard from "./Part/MainCard.svelte";
  import { filterValues, by } from "./lib/mapable";
  import { category } from "./lib/types";
  // import ShowMore from "./Widgets/ShowMore.svelte";
  // import SetDefault from "./Activity/SetDefault.svelte";
  import { parts } from "./lib/part";
  import { activities } from "./lib/activity";

  let show_more: boolean = $state(false);

  let gears = $derived(
    filterValues($parts, (p) => p.what == $category.id && !p.disposed_at).sort(
      by("last_used"),
    ),
  );
  let bin = $derived(
    filterValues(
      $parts,
      (p) => p.what == $category.id && p.disposed_at != undefined,
    ).sort(by("last_used")),
  );
</script>

{#if $category}
  <!-- <SetDefault type={$category}></SetDefault> -->
  <div class="grid grid-cols-1 sm:grid-cols-2 xl:grid-cols-3 gap-4">
    {#each gears as part (part.id)}
      <MainCard {part} />
    {:else}
      {#if $category.activities($activities).length == 0}
        We did not find any {$category.name} activities on Strava (yet).
      {:else}
        You have no {$category.name} assigned to any activity on Strava. Please do
        so to get started.
      {/if}
    {/each}
  </div>
{:else}
  Error: Category not found!
{/if}
