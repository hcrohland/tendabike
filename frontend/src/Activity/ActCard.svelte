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
  import { parts } from "../lib/part";
  import UsageChips from "../Usage/UsageChips.svelte";
  import Menu from "../Widgets/Menu.svelte";
  import { DropdownItem } from "flowbite-svelte";
  import { actions } from "../Widgets/Actions.svelte";
  import * as m from "../../paraglide/messages";

  let { activity }: { activity: Activity } = $props();

  let usage = $derived(
    new Usage({
      count: activity.count,
      climb: activity.climb || 0,
      descend: activity.descend || activity.climb || 0,
      distance: activity.distance || 0,
      time: activity.time || activity.duration || 0,
      duration: activity.duration || activity.time || 0,
      energy: activity.energy || 0,
    }),
  );
</script>

<div class="rounded-lg border border-border-subtle bg-surface-1 p-4">
  <!-- Header Row: Start — Title — Gear | Device -->
  <div class="flex items-start justify-between gap-2">
    <!-- Left: Start — Title — Gear + Device (flex-wrap on mobile) -->
    <div class="min-w-0 flex-1">
      <div class="flex items-center flex-wrap gap-x-1">
        <!-- Start -->
        <span class="text-sm text-text-1 shrink-0">
          {activity.start.toLocaleDateString()}
          {activity.start.toLocaleTimeString()}
        </span>
        <!-- Separator (desktop only, shows gap between Start and Title) -->
        <span class="hidden md:inline text-text-1"> — </span>
        <!-- Title -->
        <a
          href="/strava/activities/{activity.id}"
          target="_blank"
          class="text-sm font-bold truncate"
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
      <!-- Gear + Device row (wraps on mobile, shows on both sizes) -->
      <div class="flex items-center flex-wrap gap-x-2 mt-1">
        <!-- Desktop separator before Gear -->
        <span class="hidden md:inline text-text-1"> — </span>
        <!-- Gear -->
        <span class="min-w-0 truncate text-sm">
          {@html activity.gear && $parts[activity.gear]
            ? $parts[activity.gear].partLink()
            : "-"}
        </span>
        <!-- Gap between Gear and Device -->
        <span class="text-text-1"> | </span>
        <!-- Device (smaller font) -->
        <span
          class="text-xs text-text-1 shrink-0 truncate"
          title={activity.device_name}
        >
          {activity.device_name || "-"}
        </span>
      </div>
    </div>
    <!-- Edit menu -->
    <Menu>
      <DropdownItem onclick={() => $actions.changeActivity(activity)}>
        {m.action_change()}
      </DropdownItem>
    </Menu>
  </div>

  <!-- Usage Chips -->
  <UsageChips
    {usage}
    ref={activity.id}
    light
    gridclass="grid grid-cols-3 md:grid-cols-6 gap-2"
  />
</div>
