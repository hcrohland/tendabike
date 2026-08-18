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
  import { link } from "svelte-spa-router";
  import ServiceBadge from "./ServiceBadge.svelte";

  let {
    label,
    value,
    href = undefined,
    light = false,
    service = undefined,
    onclick = undefined,
    indicator = undefined,
  }: {
    label: string;
    value: string;
    href?: string;
    light?: boolean;
    service?: { due: number; plan: number };
    onclick?: () => void;
    indicator?: string;
  } = $props();

  let background = $derived(light ? "bg-surface-2" : "bg-surface-1");
  let isButton = $derived(!!onclick);
  let isLinked = $derived(!!href);
</script>

{#snippet chipContent()}
  <ServiceBadge {service} pos="absolute -top-3 -right-0" />
  <span class="font-semibold text-sm"> {value} </span>
  <span class="font-normal text-xs uppercase tracking-wide text-text-1">
    {label}
  </span>
  {#if indicator}
    <span class="text-xs font-bold text-primary"> {indicator} </span>
  {/if}
{/snippet}

{#if isButton && isLinked}
  <button
    {onclick}
    class="cursor-pointer focus:outline-none focus:ring-2 focus:ring-primary rounded-lg {background} shrink-0"
  >
    <a
      {href}
      use:link
      class="flex items-center gap-1 px-2 py-1 sm:px-3 sm:py-2 text-reset no-underline w-full"
      onclick={(e) => e.stopPropagation()}
    >
      {@render chipContent()}
    </a>
  </button>
{:else if isButton}
  <button
    {onclick}
    class="cursor-pointer focus:outline-none focus:ring-2 focus:ring-primary rounded-lg {background} shrink-0"
  >
    <div
      class="flex items-center gap-1 px-2 py-1 sm:px-3 sm:py-2 w-full relative"
    >
      {@render chipContent()}
    </div>
  </button>
{:else if isLinked}
  <div class="relative rounded-lg shrink-0 {background}">
    <a
      {href}
      use:link
      class="flex items-center gap-1 px-2 py-1 sm:px-3 sm:py-2 text-reset no-underline"
    >
      {@render chipContent()}
    </a>
  </div>
{:else}
  <div class="relative rounded-lg shrink-0 {background}">
    <div class="flex items-center gap-1 px-2 py-1 sm:px-3 sm:py-2">
      {@render chipContent()}
    </div>
  </div>
{/if}
