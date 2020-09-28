<script context="module">
	import {Spinner} from "sveltestrap"
	import Router from 'svelte-spa-router';

	import ToyGroup from "./ToyGroup.svelte"
	import Header from "./Header.svelte"
	import Gear from "./Gear.svelte"
	import Part from "./Part.svelte"
	import Spares from "./Spares.svelte"
	import Home from "./Home.svelte"
	import About from "./About.svelte"
	import {initData} from './store.js'

	const routes = {
		'/about': About,
		'/': Home,
    '/cat/:category': ToyGroup,
		'/gear/:id': Gear,
		'/spares/:category': Spares,
		'/part/:id': Part
	}

</script>


<Header/>
<div class="container">
	{#await initData()}
		<Spinner />
	{:then}
		<Router {routes} />
	{:catch error}
		<About />
	{/await}
</div>