<script lang="ts">
	let categories: number[];
	let promise = myfetch("/activ/categories").then(
		(data) => (categories = data),
	);
	import { myfetch, handleError, types } from "../lib/store";
	import {
		Dropdown,
		DropdownItem,
		DropdownMenu,
		DropdownToggle,
		Spinner,
	} from "@sveltestrap/sveltestrap";
	import { category } from "../lib/store";
</script>

<Dropdown direction="left">
	<DropdownToggle class="dropdown-item">Switch Sport</DropdownToggle>
	<DropdownMenu class="dropdown-submenu">
		{#await promise}
			<DropdownItem>
				<Spinner />
			</DropdownItem>
		{:then}
			{#each categories.sort() as cat}
				<DropdownItem on:click={() => category.set(types[cat])}>
					{types[cat].name}s
				</DropdownItem>
			{/each}
		{:catch error}
			{handleError(error)}
		{/await}
	</DropdownMenu>
</Dropdown>
