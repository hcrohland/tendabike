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
  import ActCard from "./ActCard.svelte";
  import TotalsCard from "./TotalsCard.svelte";
  import { Alert } from "flowbite-svelte";
  import { DAY } from "../lib/store";
  import { by } from "../lib/mapable";
  import { parts } from "../lib/part";
  import * as m from "../../paraglide/messages";

  let { acts }: { acts: Activity[] } = $props();

  // Date range state (owned by ActList, bound to TotalsCard RangeSlider)
  let dateValues = $state<number[]>([0, 0]);
  let min = $state(0);
  let max = $state(1);

  // Filter and sort state (owned by ActList, bound to TotalsCard controls)
  let searchText = $state("");
  let gearFilter = $state<number | undefined>(undefined);
  let sortBy = $state("start");
  let sortDir = $state(-1);

  // Calculate min/max days from all activities
  $effect(() => {
    if (acts.length === 0) {
      min = 0;
      max = 1;
      dateValues = [0, 1];
      return;
    }
    let set = acts.map((a) => Math.floor(a.start.getTime() / DAY));
    max = Math.max(...set);
    min = Math.min(...set);
    // Ensure min < max for RangeSlider
    if (min === max) {
      max += 1;
    }
    dateValues = [min, max];
  });

  // Date filter: filter activities by date range
  let filteredByDate = $derived(
    acts.filter((a) => {
      let rangeStart = new Date(dateValues[0] * DAY);
      let rangeEnd = new Date(dateValues[1] * DAY);
      let t = a.start.getTime();
      return rangeStart.getTime() <= t && rangeEnd.getTime() + DAY > t;
    }),
  );

  // Gear filter: filter date-filtered activities by gear
  let filteredByGear = $derived(
    filteredByDate.filter((a) => !gearFilter || a.gear === gearFilter),
  );

  // Search filter: filter by name and device_name
  let filteredBySearch = $derived(
    filteredByGear.filter((a) => {
      if (!searchText) return true;
      let search = searchText.toLowerCase();
      return (
        a.name.toLowerCase().includes(search) ||
        (a.device_name && a.device_name.toLowerCase().includes(search))
      );
    }),
  );

  // Sort activities by column and direction
  function sortActivities(
    activities: Activity[],
    key: string,
    dir: number,
  ): Activity[] {
    return [...activities].sort((a: Activity, b: Activity) => {
      let valA: any = a[key as keyof Activity];
      let valB: any = b[key as keyof Activity];
      if (key === "descend") {
        if (valA === undefined) valA = a.climb || 0;
        if (valB === undefined) valB = b.climb || 0;
      }

      // Handle undefined/null values
      if (valA == null) return dir;
      if (valB == null) return -dir;

      // Date comparison for 'start'
      if (key === "start") {
        return dir * (a.start.getTime() - b.start.getTime());
      }

      // Numeric comparison for time, distance, climb, descend, energy
      if (["time", "distance", "climb", "descend", "energy"].includes(key)) {
        return dir * (valA - valB);
      }

      // String comparison for name, device_name
      return dir * valA.toString().localeCompare(valB.toString());
    });
  }

  // Reactive pipeline: acts -> dateFilter -> gearFilter -> search -> sort -> display
  let displayed = $derived(sortActivities(filteredBySearch, sortBy, sortDir));

  // Gear filter options: derived from all activities (not filtered), stable across filter changes
  let filterOptions = $derived(
    (() => {
      let types: Record<string, any> = {};
      acts.forEach((act) => {
        let key = String(act.gear || 0);
        if (types[key] === undefined) {
          if (act.gear) {
            let part = $parts[act.gear];
            let name = part ? part.name : "-";
            types[key] = { name, value: act.gear, start: act.start };
          } else {
            types[key] = {
              name: m.filter_none_option(),
              value: 0,
              start: new Date(),
            };
          }
        }
      });
      let res = Object.values(types).sort(by<any>("start"));
      return res.length >= 1 ? res : undefined;
    })(),
  );
</script>

<div class="flex flex-col gap-4 max-w-4xl mx-auto">
  <!-- Totals & Controls (date slider + filters + sort) -->
  <TotalsCard
    activities={displayed}
    {filterOptions}
    bind:dateValues
    bind:min
    bind:max
    bind:gearFilter
    bind:searchText
    bind:sortBy
    bind:sortDir
  />

  <!-- Activity Cards -->
  {#if acts.length === 0}
    <Alert color="secondary">{m.act_no_activities()}</Alert>
  {:else if displayed.length === 0}
    <Alert color="secondary">{m.act_no_activities()}</Alert>
  {:else}
    {#each displayed as act (act.id)}
      <ActCard activity={act} />
    {/each}
  {/if}
</div>
