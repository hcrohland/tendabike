<script>
	import {beforeUpdate, onDestroy} from 'svelte'
	import {gear, category} from "./store.js";
	import ToyGroup from "./ToyGroup.svelte"
				
	export let params = {};

	beforeUpdate (() => category.set(params.cat))
	onDestroy (() => category.set(null))
</script>
{#if $gear[params.cat]}
	<ToyGroup parts={$gear[params.cat]} />
{:else}
	<div class="alert alert-info" role="alert">
		Please select the Gear you wanna tend!
	</div>
{/if}
