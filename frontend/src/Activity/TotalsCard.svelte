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
  import UsageChips from "../Usage/UsageChips.svelte";
  import { DAY } from "../lib/store";
  import RangeSlider from "svelte-range-slider-pips";
  import * as m from "../../paraglide/messages";

  type FilterOption = {
    name: string;
    value: number;
    start: Date;
  };

  type Props = {
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

  // Sort options
  type SortOption = { key: string; label: string };
  const sortOptions: SortOption[] = [
    { key: "start", label: m.act_col_start() },
    { key: "name", label: m.partform_name() },
    { key: "time", label: m.act_col_time() },
    { key: "distance", label: m.act_col_distance() },
    { key: "climb", label: m.limit_climb() },
    { key: "descend", label: m.limit_descend() },
    { key: "energy", label: m.limit_kJ() },
  ];
</script>

<div
  class="rounded-lg border border-border-subtle bg-surface-2 p-4 sticky top-16 z-10"
>
  <!-- Totals Row -->
  <div class="mb-3">
    <div class="text-sm font-bold uppercase text-text-1 mb-2">
      {m.act_total()}
    </div>
    <UsageChips
      usage={totals}
      gridclass="grid grid-cols-3 md:grid-cols-6 gap-2"
    />
  </div>

  <!-- Filters Row -->
  <div class="flex flex-wrap gap-2 items-center mb-3">
    <!-- Search Input -->
    <input
      type="text"
      bind:value={searchText}
      class="text-sm p-1 rounded bg-surface-1 border border-border-subtle w-full md:w-48"
      placeholder="Search name or device..."
    />

    <!-- Gear Filter -->
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

    <!-- Sort By -->
    <select
      value={sortBy}
      oninput={(e) => {
        sortBy = e.currentTarget.value;
      }}
      class="text-sm p-1 rounded bg-surface-1 border border-border-subtle"
    >
      {#each sortOptions as opt}
        <option value={opt.key}>{opt.label}</option>
      {/each}
    </select>

    <!-- Sort Order Toggle -->
    <button
      onclick={() => {
        sortDir = sortDir === -1 ? 1 : -1;
      }}
      class="text-sm p-1 px-2 rounded bg-surface-1 border border-border-subtle"
      title={sortDir === -1 ? "▼ Desc" : "▲ Asc"}
    >
      {sortDir === -1 ? "▼" : "▲"}
    </button>
  </div>

  <!-- Date Range Slider -->
  <RangeSlider
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
