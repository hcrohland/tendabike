<script lang="ts" context="module">
	import {Spinner, Container} from "sveltestrap"
	import Router from 'svelte-spa-router';

	import ToyGroup from "./ToyGroup.svelte"
	import Header from "./Header.svelte"
	import Gear from "./Gear/Gear.svelte"
	import Part from "./Part/PartView.svelte"
	import Spares from "./Spares/Spares.svelte"
	import Home from "./Home.svelte"
	import About from "./About.svelte"
	import {initData} from './store'

	const routes = {
		'/about': About,
		'/': ToyGroup,
    '/cat/:category': ToyGroup,
		'/gear/:id': Gear,
		'/spares/:category': Spares,
		'/part/:id': Part,
		'/switch': Home
	}

</script>


<Header/>
<Container class="mt-2">
	{#await initData()}
		<Spinner />
	{:then}
		<Router {routes} restoreScrollState={true}/>
	{:catch error}
		<About />
	{/await}
</Container>