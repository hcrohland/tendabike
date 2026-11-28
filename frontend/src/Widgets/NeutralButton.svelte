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
  // A neutral button: a plain element carrying the theme's single uniform
  // fill role class, like a chip. The component library's button is not
  // used — it composes its color classes from a hard-coded internal table
  // by plain concatenation, and a theme class cannot override them, so a
  // neutral button is a plain element (ADR-0019). Status buttons remain
  // library-native.
  let {
    onclick = undefined,
    href = undefined,
    class: extraClass = "",
    type = "button",
    value = undefined,
    disabled = false,
    size = "md",
    children,
    ...rest
  }: {
    onclick?: (e: MouseEvent) => void;
    href?: string;
    class?: string;
    type?: "button" | "submit" | "reset";
    value?: string;
    disabled?: boolean;
    size?: "xs" | "sm" | "md";
    children: import("svelte").Snippet;
    [key: string]: any;
  } = $props();

  // The standard neutral button shape (the library's sizing steps): the
  // color is the theme's fill, not a per-color step.
  const shape = $derived(
    size === "xs"
      ? "px-3 py-2 text-xs"
      : size === "sm"
        ? "px-4 py-2 text-sm"
        : "px-5 py-2.5 text-sm",
  );

  const cls = $derived(
    "fill text-center font-medium inline-flex items-center justify-center gap-1 rounded-lg " +
      shape +
      (href
        ? ""
        : " cursor-pointer focus:outline-none focus:ring-2 focus:ring-secondary-500") +
      (extraClass ? " " + extraClass : ""),
  );
</script>

{#if href}
  <a {href} {...rest} class={cls}>
    {@render children()}
  </a>
{:else}
  <button {type} {value} {disabled} {onclick} {...rest} class={cls}>
    {@render children()}
  </button>
{/if}
