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
  import { parts } from "../lib/part";
  import Chip from "../Widgets/Chip.svelte";
  import Menu from "../Widgets/Menu.svelte";
  import { DropdownItem } from "flowbite-svelte";
  import { actions } from "../Widgets/Actions.svelte";
  import { fmtNumber, fmtSeconds } from "../lib/store";
  import * as m from "../../paraglide/messages";

  let { activity }: { activity: Activity } = $props();

  // Gear part derived from activity
  let gearPart = $derived(
    activity.gear && $parts[activity.gear] ? $parts[activity.gear] : null,
  );
</script>

<div class="rounded-lg border border-border-subtle bg-surface-1 p-2 sm:p-3">
  <!-- Header Row: 4-column grid layout (same on mobile and desktop) -->
  <!-- Col 1-3 (col-span-3): Date + Time + Activity Name (wraps freely) -->
  <!-- Col 4 (col-span-1): Device name (right-aligned) + Menu (shrink-0) -->
  <!-- Layout: [Date — Time + Title ...........] [Device] [Menu] -->
  <div class="grid grid-cols-4 items-center gap-1 sm:gap-2 mb-2 sm:mb-3">
    <!-- Columns 1-3: Date + Time + Activity Name -->
    <div class="col-span-3 min-w-0">
      <div class="flex items-center gap-x-0.5 sm:gap-x-1">
        <!-- Date + Time -->
        <span class="text-sm text-text-1 shrink-0">
          {activity.start.toLocaleDateString()}
          {activity.start.toLocaleTimeString()}
        </span>
        <!-- Gap between Start and Title (em-dash on desktop, thin space on mobile) -->
        <span class="hidden sm:inline text-text-1"> — </span>
        <span class="sm:hidden text-text-1">&thinsp;</span>
        <!-- Title (wraps on long names) -->
        <a
          href="/strava/activities/{activity.id}"
          target="_blank"
          class="text-sm font-bold wrap-break-word"
        >
          {activity.name}
          <img
            src="strava_grey.png"
            alt={m.gearcard_view_on_strava()}
            title={m.gearcard_view_on_strava()}
            class="inline mx-1 w-4 h-4 align-middle"
          />
        </a>
      </div>
    </div>
    <!-- Column 4: Device (right-aligned, truncates) + Menu -->
    <div class="col-span-1 flex items-center justify-end gap-1 shrink-0">
      <!-- Device name (right-aligned via sm:text-right, truncates on overflow) -->
      <span
        class="text-xs text-text-1 truncate sm:text-right"
        title={activity.device_name}
      >
        {activity.device_name || "-"}
      </span>
      <!-- Edit Menu -->
      <Menu>
        <DropdownItem onclick={() => $actions.changeActivity(activity)}>
          {m.action_change()}
        </DropdownItem>
      </Menu>
    </div>
  </div>

  <!-- Metrics Grid: Gear | Time | Distance | Climb | Descend | Energy -->
  <div class="grid grid-cols-3 sm:grid-cols-6 gap-1 sm:gap-2">
    <!-- Gear Chip (link) -->
    <Chip
      value={gearPart?.name ?? "-"}
      label=""
      href={gearPart?.link()}
      light
    />
    <!-- Time -->
    <Chip
      value={fmtSeconds(activity.time || activity.duration || 0)}
      label="h"
      light
    />
    <!-- Distance -->
    <Chip
      value={fmtNumber(Math.round((activity.distance || 0) / 1000))}
      label="km"
      light
    />
    <!-- Climb -->
    <Chip value={fmtNumber(activity.climb || 0)} label="↑m" light />
    <!-- Descend -->
    <Chip
      value={fmtNumber(activity.descend || activity.climb || 0)}
      label="↓m"
      light
    />
    <!-- Energy (only if > 0) -->
    {#if (activity.energy || 0) > 0}
      <Chip value={fmtNumber(activity.energy || 0)} label="kJ" light />
    {/if}
  </div>
</div>
