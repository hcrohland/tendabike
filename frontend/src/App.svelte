<script lang="ts" context="module">
	import {Spinner, Container} from "sveltestrap"
	import Router from 'svelte-spa-router';

	import ToyGroup from "./ToyGroup.svelte"
	import Header from "./Header.svelte"
	import Gear from "./Part/Part.svelte"
	import Spares from "./Spares/Spares.svelte"
	import Home from "./Home.svelte"
	import About from "./About.svelte"
  import Message from './Message.svelte'
  import Admin from './Admin/Admin.svelte'
	import {initData} from './store'
  import Statistics from "./Statistics.svelte";

	const routes = {
		'/about': About,
		'/': ToyGroup,
    '/cat/:category': ToyGroup,
		'/part/:id': Gear,
		'/spares/:category': Spares,
		'/switch': Home,
		'/admin': Admin,
		'/stats': Statistics
	}

</script>


<Header/>
<Container class="mt-2">
	{#await initData()}
		<Spinner />
	{:then}
	  <Message />
		<Router {routes} restoreScrollState={true}/>
	{:catch error}
		<About />
	{/await}
</Container>