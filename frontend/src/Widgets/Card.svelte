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
  let {
    light = false,
    class: extraClass = "",
    children,
  }: {
    light?: boolean;
    class?: string;
    children: import("svelte").Snippet;
  } = $props();

  let background = $derived(light ? "bg-surface-1" : "bg-surface-2");
</script>

<div
  class={"rounded-lg border border-border-subtle p-1 sm:p-2 " +
    background +
    (extraClass ? " " + extraClass : "")}
>
  {@render children()}
</div>
