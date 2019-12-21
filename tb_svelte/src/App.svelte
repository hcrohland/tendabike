<script context="module">
	import _ from 'lodash';
	import Router from 'svelte-spa-router';

	import ToyGroup from "./ToyGroup.svelte"
	import Await from "./Await.svelte"
	import Header from "./Header.svelte"
	import Part from "./Part.svelte"
	import Home from "./Home.svelte"
	import About from "./About.svelte"
	import {myfetch, types, parts} from './store.js'

	const routes = {
		'/about': About,
		'/': Home,
    '/cat/:category': ToyGroup,
    '/part/:id': Part
	}

	let promise = Promise.all([
			myfetch('/types/part')
				.then(data => types.setMap(data)),
			myfetch('/part/all')
				.then(data => parts.setMap(data))
	])
</script>


<Header/>
<Await {promise}>
	<div class="container">
		<Router {routes} />
	</div>
</Await>