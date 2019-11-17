<script>
	import Api from "./Api.svelte";
	import ToyGroup from "./ToyGroup.svelte"
  	import 	{ Container, Button, Card, CardBody, CardFooter, CardHeader,
    			CardImg, CardSubtitle, CardText, CardTitle, Badge
			} from 'sveltestrap';
	import { onMount, setContext } from 'svelte';
	import _ from 'lodash';

	let types = [];
	setContext("types", {
		getTypes: () => types
	})

	onMount(async () => {
		const res = await fetch(`http://localhost:8000/types/part`);
		types = await res.json();
	});

</script>


<style>
	h1 {
		Color: rgba(0, 0, 0, 0.532);
	}

	.card {
		margin: 1rem;
	}
</style>
	
<!-- <ul>
{#each Object.entries(types) as [id, type]}
	<li>  {id} - {type.name}
{/each}
<li> </li>
</ul>
 -->

<Api url="http://localhost:8000/part/mygear" let:data>
	{#each Object.entries(_.groupBy(data, "what")) as [group, parts]}
		<ToyGroup {group} {parts} />
	{/each}
</Api>
