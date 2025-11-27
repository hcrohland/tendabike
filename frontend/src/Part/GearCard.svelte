<script lang="ts">
  // import { slide } from 'svelte/transition';
  import { Card } from "flowbite-svelte";
  import { link, push } from "svelte-spa-router";
  import { Part } from "../lib/part";
  import { fmtDate, fmtNumber, fmtSeconds } from "../lib/store";
  import { types } from "../lib/types";
  import { Usage, usages } from "../lib/usage";

  interface Props {
    part: Part;
    summary?: boolean;
    children?: import("svelte").Snippet;
  }

  let { part, summary = false, children }: Props = $props();

  let usage = $derived($usages[part.usage] ? $usages[part.usage] : new Usage());

  function model(part: Part) {
    if (part.model == "" && part.vendor == "") {
      return "unknown model";
    } else {
      return part.vendor + " " + part.model;
    }
  }
</script>

<Card size="xl" class="col-auto">
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class={"text-xl bg-gray-200 dark:bg-gray-700 p-4" +
      (summary
        ? " hover:bg-gray-300 dark:hover:bg-gray-500 cursor-pointer"
        : "")}
    onclick={() => summary && push("/part/" + part.id)}
  >
    {#if summary}
      <a href="/part/{part.id}" use:link class="text-decoration-none">
        {part.name}
      </a>
    {:else}
      {part.name}
    {/if}
    <div class="float-end h6 mb-0">
      {@render children?.()}
    </div>
  </div>
  <div class="text-wrap p-4">
    is a <span class="param">{model(part)}</span>
    {#if part.what == 1}
      <a href={"/strava/bikes/" + part.id} target="_blank">
        <img
          src="strava_grey.png"
          alt="View on Strava"
          title="View on Strava"
          class="inline"
        />
      </a>
    {/if}
    {#if part.what != types[part.what].main}
      {types[part.what].name.toLowerCase()}
    {/if}
    {#if !part.disposed_at}
      purchased <span class="param">{fmtDate(part.purchase)}</span>
      which
    {:else}
      you owned from <span class="param">{fmtDate(part.purchase)}</span>
      until <span class="param">{fmtDate(part.disposed_at)}</span>
      and
    {/if}
    you used
    <a href={"/activities/" + part.id} use:link class="param text-reset">
      {fmtNumber(usage.count)}
    </a>
    times for <span class="param">{fmtSeconds(usage.time)}</span> hours.
    <p>
      You covered <span class="param"
        >{fmtNumber(parseFloat((usage.distance / 1000).toFixed(1)))}</span
      >
      km climbing <span class="param">{fmtNumber(usage.climb)}</span> and
      descending <span class="param">{fmtNumber(usage.descend)}</span> meters
      {#if usage.energy > 0}
        and expended
        <span class="param">{fmtNumber(usage.energy)}</span> kiloJoules of energy
      {/if}
    </p>
    {#if !summary && part.notes}
      <div class="mt-3">
        <strong>Notes:</strong>
        <p class="text-gray-700 dark:text-gray-300 whitespace-pre-wrap">
          {part.notes}
        </p>
      </div>
    {/if}
  </div>
</Card>
