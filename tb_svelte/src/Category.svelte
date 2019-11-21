<script>
	import {gear, category} from "./store.js";
	import ToyGroup from "./ToyGroup.svelte"
				
	export let params = {};

	$: category.set(params.cat)
</script>
{#if $gear[params.cat]}
	<ToyGroup parts={$gear[params.cat]} />
{:else}
	<div class="alert alert-info" role="alert">
		Please select the Gear you wanna tend!
	</div>
{/if}
