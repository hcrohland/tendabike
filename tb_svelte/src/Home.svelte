<script>
import {myfetch, handleError, types, icons, parts, category} from './store';
import {Spinner} from 'sveltestrap'

category.set(undefined);

let promise; 
$: { $parts; promise = myfetch('/activ/categories') }
</script>

{#await promise}
	<Spinner />
{:then categories}
	<div class="row justify-content-around m-0 p-0">
		{#each categories as cat}
			<div class="col text-center p-0 p-sm-2 {icons[cat] ? 'order-1' : 'col-12 order-2'}">
				<div class="card">
					<div class="card-header">
						<a class="card-link stretched-link text-dark text-center" href="#/cat/{cat}" title={$types[cat].name}s> 
							{#if icons[cat]}
								<span class="{icons[cat]}" style="font-size: 60px"></span> 
							{:else}
								<strong> {$types[cat].name}s </strong>
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