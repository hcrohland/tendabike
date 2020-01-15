<script context="module">
	import _ from 'lodash';
	import Router from 'svelte-spa-router';

	import ToyGroup from "./ToyGroup.svelte"
	import Await from "./Await.svelte"
	import Header from "./Header.svelte"
	import Gear from "./Gear.svelte"
	import Spares from "./Spares.svelte"
	import Home from "./Home.svelte"
	import About from "./About.svelte"
	import {myfetch, types, parts, user} from './store.js'

	const routes = {
		'/about': About,
		'/': Home,
    '/cat/:category': ToyGroup,
		'/gear/:id': Gear,
		'/spares/:category': Spares
	}

	let promise = Promise.all([
			myfetch('/types/part')
				.then(data => types.setMap(data)),
			myfetch('/part/all')
				.then(data => parts.setMap(data)),
			myfetch('/user')
				.then(data => user.set(data))
	])
</script>


<Header/>
<Await {promise}>
	<div class="container">
		<Router {routes} />
	</div>
</Await>