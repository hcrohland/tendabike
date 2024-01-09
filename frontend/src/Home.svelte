<script lang="ts">
    import { link } from 'svelte-spa-router';
import {myfetch, handleError, types, icons, parts, category} from './lib/store';
import {Spinner} from '@sveltestrap/sveltestrap'

category.set(undefined);

let promise: Promise<void>, categories: number[]; 
$: { $parts; promise = myfetch('/activ/categories').then(data => categories = data) }
</script>

{#await promise}
	<Spinner />
{:then}
	<div class="row justify-content-around m-0 p-0">
		{#each categories as cat}
			<div class="col text-center p-0 p-sm-2 {icons.get(cat) ? 'order-1' : 'col-12 order-2'}">
				<div class="card">
					<div class="card-header">
						<a class="card-link stretched-link text-dark text-center" use:link href="/cat/{cat}" title={types[cat].name}s> 
							{#if icons.get(cat)}
								<span class="{icons.get(cat)}" style="font-size: 60px"></span> 
							{:else}
								<strong> {types[cat].name}s </strong>
							{/if}
						</a>
					</div>
				</div>
			</div>
		{/each}
	</div>
{:catch error}
	{handleError(error)}
{/await}