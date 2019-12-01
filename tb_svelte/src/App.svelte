<script>
	import _ from 'lodash';
	import Router from 'svelte-spa-router';

	import ToyGroup from "./ToyGroup.svelte"
	import Header from "./Header.svelte"
	import Home from "./Home.svelte"
	import About from "./About.svelte"
	import myfetch, {types} from './store.js'

	const routes = {
		'/about': About,
		'/': Home,
    '/:category': ToyGroup
	}

	let promise = myfetch('/types/part')
			.then(data => types.set(data));
</script>


<Header/>
{#await promise}
	loading data...
{:then data}
	<div class="container">
		<Router {routes} />
	</div>
{:catch error}
	<div class="alert alert-warning" role="alert">
		<h2>
			{error}
		</h2>
	</div>
{/await}