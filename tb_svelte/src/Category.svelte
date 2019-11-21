<script>
	import {gear, category} from "./store.js";
	import ToyGroup from "./ToyGroup.svelte"
</script>

{#if $gear[$category]}
	<ToyGroup parts={$gear[$category]} />
{:else}
	<div class="alert alert-info" role="alert">
		Please select the Gear you wanna tend!
	</div>
{/if}
