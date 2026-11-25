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
  import { getContext, setContext, untrack } from "svelte";
  import { ToneContext, bg, opposite, type Tone } from "./tone.js";

  let {
    light = undefined,
    class: extraClass = "",
    children,
  }: {
    light?: boolean;
    class?: string;
    children: import("svelte").Snippet;
  } = $props();

  // A Card's tone. The explicit `light` prop wins and keeps its current
  // direction (light ⇒ the light tone, dark ⇒ the darker tone); otherwise a
  // Card flips the tone its parent Card provides, and a top-level Card
  // defaults to the darker tone.
  const parentTone = getContext(ToneContext) as Tone | undefined;
  const tone: Tone = $derived(
    light === true
      ? "primary"
      : light === false
        ? "secondary"
        : parentTone
          ? opposite(parentTone)
          : "secondary",
  );

  // The tone is fixed once the Card mounts (it depends only on the `light`
  // prop and the parent's tone), so publish the resolved value, not a live ref.
  setContext(
    ToneContext,
    untrack(() => tone),
  );

  const background = $derived(bg(tone));
</script>

<div
  class={"rounded-lg border border-secondary-400 dark:border-secondary-700 p-1 sm:p-2 " +
    background +
    (extraClass ? " " + extraClass : "")}
>
  {@render children()}
</div>
