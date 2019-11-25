<script>
import myfetch, {types, icons, category} from './store.js';

let promise = myfetch('/part/categories')
$category = null;
</script>

{#await promise}
	loading
{:then categories}
	<div class="row justify-content-around m-0 p-0">
		{#each categories as cat}
			<div class="col text-center p-0 p-sm-2 {icons[cat] ? 'order-1' : 'col-12 order-2'}">
				<div class="card">
					<div class="card-header">
						<h5 class="mb-0"> 
							<a class="card-link stretched-link text-dark text-center" href="#/{cat}" title={$types[cat].name}s> 
								{#if icons[cat]}
									<span class="{icons[cat]}" style="font-size: 60px"></span> 
								{:else}
									<strong> {$types[cat].name}s </strong>
								{/if}
							</a>
						</h5>
					</div>
				</div>
			</div>
		{/each}
	</div>
{:catch error}
  {error}
{/await}
