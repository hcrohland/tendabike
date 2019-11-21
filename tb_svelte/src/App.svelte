<script>
	import _ from 'lodash';
	import Router from 'svelte-spa-router';

	import Category from "./Category.svelte"
	import Header from "./Header.svelte"
	import About from "./About.svelte"
	import fetch_store, {category} from './store.js'

	const routes = {
		'/about': About,
    '/': Category
	}

	let prom = fetch_store();
</script>

<Header />

{#await prom}
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