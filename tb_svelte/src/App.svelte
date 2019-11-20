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
		<div class="navbar navbar-expand-md bg-secondary mb-3">
			<div class="navbar-brand">
				TendAGear
			</div>
			<button class="navbar-toggler" type="button" data-toggle="collapse" data-target="#navbarSupportedContent" aria-controls="navbarSupportedContent" aria-expanded="false" aria-label="Toggle navigation">
    		<span class="navbar-toggler-icon"></span>
  		</button>	
		</div>
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