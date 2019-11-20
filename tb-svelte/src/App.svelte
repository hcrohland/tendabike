<script>
	import _ from 'lodash';

	import {types} from "./store.js";
	import ToyGroup from "./ToyGroup.svelte"
	import Bar from "./Bar.svelte"

	function handleErrors(response) {
    if (!response.ok) {
        throw Error(response.status + ' "' + response.statusText + '" accessing ' + response.url);
    }
    return response;
	}


	let data_w = Promise.all([
		fetch('http://localhost:8000/types/part')
				.then(handleErrors)
				.then(response => response.json())
				.then(data => types.set(data)),
			fetch(`http://localhost:8000/part/mygear`)
				.then(handleErrors)
				.then(response => response.json())
				.then(data => gear = _.groupBy(data, "what"))
	])
				
	let gear = [];
	let category = "1";
	$: categories = Object.keys(gear) || [];
	$: parts = gear[category] || []
</script>

<div class="container">
{#await data_w}
	loading data...
{:then data}
		<Bar bind:category {categories}/>
		<ToyGroup {parts} />
{:catch error}
	<div class="alert alert-warning" role="alert">
		<h2>
			{error}
		</h2>
	</div>
{/await}
</div>