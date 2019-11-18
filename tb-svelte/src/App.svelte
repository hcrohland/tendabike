<script>
	import {types, gear, category, parts, categories} from "./store.js";
	import ToyGroup from "./ToyGroup.svelte"
	import Bar from "./Bar.svelte"

	import { onMount, setContext } from 'svelte';
	import _ from 'lodash';

	let w_types = fetch('http://localhost:8000/types/part')
		.then(response => response.json())
		.then(data => types.set(data));
</script>


<style>

</style>
	
<!-- <ul>
{#each Object.entries(types) as [id, type]}
	<li>  {id} - {type.name}
{/each}
<li> </li>
</ul>
 -->
{#await w_types}
	loading
{:then data}
	<Bar />

		<ToyGroup group={$category} parts={$parts} />
{/await}
