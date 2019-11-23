<script>
import myfetch, {types, category} from './store.js';

let promise = myfetch('/part/categories')
$category = null;
</script>

{#await promise}
	loading
{:then categories}
	<div class="row m-0 p-0">
		{#each categories as cat}
			<div class="col-md-6 p-0 p-sm-2">
				<div class="card">
					<div class="card-header">
						<h5 class="mb-0"> 
							<a class="card-link stretched-link text-dark" href="#/{cat}"> {$types[cat].name}s </a>
						</h5>
					</div>
				</div>
			</div>
		{/each}
	</div>
{:catch error}
  {error}
{/await}
