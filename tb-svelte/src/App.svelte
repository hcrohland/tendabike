<script>
	import _ from 'lodash';

	import {types} from "./store.js";
	import ToyGroup from "./ToyGroup.svelte"
	import Bar from "./Bar.svelte"

	let data_w = Promise.all([
		fetch('http://localhost:8000/types/part')
				.then(response => response.json())
				.then(data => types.set(data)),
			fetch(`http://localhost:8000/part/mygear`)
				.then(response => response.json())
				.then(data => gear = _.groupBy(data, "what"))
	])
				
	let gear = [];
	let category = "1";
	$: categories = Object.keys(gear) || [];
	$: parts = gear[category] || []
</script>

{#await data_w}
	loading data...
{:then data}
	<Bar bind:category {categories}/>
	<ToyGroup {parts} />
{/await}
