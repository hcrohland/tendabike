<!--
	tendabike - the bike maintenance tracker
	
	Copyright (C) 2023-2026 Christoph Rohland

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
  import { colorClasses, type Color } from "./color.js";

  let {
    color = "secondary",
    class: extraClass = "",
    children,
  }: {
    color?: Color;
    class?: string;
    children: import("svelte").Snippet;
  } = $props();

  // A Card renders the surface (background + border) of its color. A
  // top-level card defaults to the darker tone; a nested card is authored
  // with the opposite color by its caller — there is no context and no
  // automatic flip.
  const surface = $derived(colorClasses[color].surface);
</script>

<div
  class={"rounded-lg border p-1 sm:p-2 " +
    surface +
    (extraClass ? " " + extraClass : "")}
>
  {@render children()}
</div>
