<script context="module">
	import _ from 'lodash';
	import Router from 'svelte-spa-router';

	import ToyGroup from "./ToyGroup.svelte"
	import Await from "./Await.svelte"
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
			.then(data => types.setMap(data));
</script>


<Header/>
<Await {promise}>
	<div class="container">
		<Router {routes} />
	</div>
</Await>