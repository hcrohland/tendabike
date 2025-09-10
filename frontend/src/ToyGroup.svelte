<script lang="ts">
  import { Col, Row } from "@sveltestrap/sveltestrap";
  import MainCard from "./Part/MainCard.svelte";
  import { filterValues, by } from "./lib/mapable";
  import { category } from "./lib/types";
  import ShowMore from "./Widgets/ShowMore.svelte";
  import SetDefault from "./Activity/SetDefault.svelte";
  import { parts } from "./lib/part";
  import { activities } from "./lib/activity";

  let show_more: boolean;

  $: gears = filterValues(
    $parts,
    (p) => p.what == $category.id && !p.disposed_at,
  ).sort(by("last_used"));
  $: bin = filterValues(
    $parts,
    (p) => p.what == $category.id && p.disposed_at != undefined,
  ).sort(by("last_used"));
</script>

{#if $category}
  <SetDefault type={$category}></SetDefault>
  <Row class="p-sm-2">
    {#each gears as part (part.id)}
      <Col md="6" class="p-0 p-sm-2">
        <MainCard {part} />
      </Col>
    {:else}
      {#if $category.activities($activities).length == 0}
        We did not find any {$category.name} activities on Strava (yet).
      {:else}
        You have no {$category.name} assigned to any activity on Strava. Please do
        so to get started.
      {/if}
    {/each}
  </Row>

  {#if bin.length > 0}
    <ShowMore bind:show_more>Show disposed</ShowMore>
    {#if show_more}
      <Row>
        {#each bin as part (part.id)}
          <Col md="6" class="p-0 p-sm-2">
            <MainCard {part} />
          </Col>
        {/each}
      </Row>
    {/if}
  {/if}
{:else}
  Error: Category not found!
{/if}
