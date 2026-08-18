<!-- 
	tendabike - the bike maintenance tracker
	
	Copyright (C) 2023  Christoph Rohland 

	This program is free software: you can redistribute it and/or modify
	it under the terms of the GNU Affero General Public License as published
	by the Free Software Foundation, either version 3 of the License, or
	(at your option) any later version.

	This program is distributed in the hope that it will be useful,
	but WITHOUT ANY WARRANTY; without even the implied warranty of
	MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
	GNU Affero General Public License for more details.

	You should have received a copy of the GNU Affero General Public License
	along with this program.  If not, see <https://www.gnu.org/licenses/>.
	
 -->

<script lang="ts">
  import { Activity } from "../lib/activity";
  import { Usage } from "../lib/usage";
  import Chip from "../Widgets/Chip.svelte";
  import { DAY, fmtNumber, fmtSeconds } from "../lib/store";
  import RangeSlider from "svelte-range-slider-pips";
  import { tick } from "svelte";
  import * as m from "../../paraglide/messages";

  type FilterOption = {
    name: string;
    value: number;
    start: Date;
  };

  type Props = {
    title: string;
    activities: Activity[];
    filterOptions: FilterOption[] | undefined;
    dateValues: number[];
    min: number;
    max: number;
    gearFilter: number | undefined;
    searchText: string;
    sortBy: string;
    sortDir: number;
  };

  let {
    title,
    activities,
    filterOptions,
    dateValues = $bindable(),
    min = $bindable(0),
    max = $bindable(0),
    gearFilter = $bindable(),
    searchText = $bindable(""),
    sortBy = $bindable("start"),
    sortDir = $bindable(-1),
  }: Props = $props();

  // Internal sorting state for loading indicator
  let sorting = $state(false);
  // Tracks which chip label is currently the sorting target
  let sortTarget = $state("");

  // Ensure defaults with $derived
  let safeActivities = $derived(activities ?? []);

  // Calculate totals from displayed activities
  let totals = $derived(
    safeActivities.reduce((total: Usage, act: Activity) => {
      total.add(act);
      return total;
    }, new Usage()),
  );

  const formatter = (v: number) => new Date(v * DAY).toLocaleDateString();

  // Map from chip label key to sort field
  const sortMapping: Record<string, string> = {
    rides: "start", // Count -> sort by date
    time: "time", // Time -> sort by duration
    distance: "distance",
    climb: "climb",
    descend: "descend",
    energy: "energy",
  };

  // Toggle sort by clicking a metric chip
  async function toggleSort(label: string) {
    sortTarget = sortMapping[label];
    if (!sortTarget) return;

    sorting = true;

    // Wait for the browser to paint at least one frame with the loading state.
    // tick() ensures Svelte processes the sorting=true state change,
    // then two requestAnimationFrame calls ensure one full paint cycle happens
    // before we apply the sort change.
    await tick();
    await new Promise((resolve) => {
      requestAnimationFrame(() => {
        requestAnimationFrame(resolve);
      });
    });

    if (sortBy === sortTarget) {
      // Same column: toggle direction (1 = asc, -1 = desc)
      sortDir = sortDir === 1 ? -1 : 1;
    } else {
      // Different column: sort descending by default
      sortBy = sortTarget;
      sortDir = -1;
    }

    // Reset sorting state after sorting completes
    sorting = false;
  }

  // Show direction indicator only for the currently sorted column
  function sortIndicator(label: string): string | undefined {
    const field = sortMapping[label];
    if (sortTarget !== field) return undefined;
    // -1 = descending (▼), 1 = ascending (▲)
    return sortDir === -1 ? "▼" : "▲";
  }
</script>

<div class="rounded-lg border border-border-subtle bg-surface-2 p-4">
  <!-- Top line: title + gear filter + search -->
  <div class="mb-3">
    <div class="flex items-center justify-between gap-2 mb-2">
      <div class="text-sm font-bold uppercase text-text-1">{title}</div>
      <div class="flex items-center gap-2">
        <!-- Gear Filter (first) -->
        {#if filterOptions}
          <select
            value={gearFilter ?? ""}
            oninput={(e) => {
              const v = e.currentTarget.value;
              gearFilter = v === "" ? undefined : Number(v);
            }}
            class="text-sm p-1 rounded bg-surface-1 border border-border-subtle"
          >
            <option value={undefined}>{m.filter_all()}</option>
            {#each filterOptions as opt}
              <option value={opt.value}>{opt.name}</option>
            {/each}
          </select>
        {/if}
        <!-- Search Input (second) -->
        <input
          type="text"
          bind:value={searchText}
          class="text-sm p-1 rounded bg-surface-1 border border-border-subtle w-24 sm:w-48"
          placeholder="Search..."
        />
      </div>
    </div>
    <!-- Chips grid -->
    <div class="grid grid-cols-3 sm:grid-cols-6 gap-2 m-4">
      <Chip
        value={fmtNumber(totals.count)}
        label={m.usage_rides()}
        onclick={() => toggleSort("rides")}
        indicator={sortIndicator("rides")}
        disabled={sorting}
      />
      <Chip
        value={fmtSeconds(totals.time)}
        label="h"
        onclick={() => toggleSort("time")}
        indicator={sortIndicator("time")}
        disabled={sorting}
      />
      <Chip
        value={fmtNumber(Math.round((totals.distance || 0) / 1000))}
        label="km"
        onclick={() => toggleSort("distance")}
        indicator={sortIndicator("distance")}
        disabled={sorting}
      />
      <Chip
        value={fmtNumber(totals.climb)}
        label="↑m"
        onclick={() => toggleSort("climb")}
        indicator={sortIndicator("climb")}
        disabled={sorting}
      />
      <Chip
        value={fmtNumber(totals.descend)}
        label="↓m"
        onclick={() => toggleSort("descend")}
        indicator={sortIndicator("descend")}
        disabled={sorting}
      />
      {#if totals.energy > 0}
        <Chip
          value={fmtNumber(totals.energy)}
          label="kJ"
          onclick={() => toggleSort("energy")}
          indicator={sortIndicator("energy")}
          disabled={sorting}
        />
      {/if}
    </div>
  </div>

  <!-- Date Range Slider -->
  <div class="mx-8 mt-6">
    <RangeSlider
      style="font-size: 0.75rem"
      {min}
      {max}
      range
      pushy
      pips
      first="label"
      last="label"
      float
      {formatter}
      bind:values={dateValues}
    />
  </div>
</div>
