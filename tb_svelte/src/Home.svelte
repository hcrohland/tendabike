<script context="module">
import myfetch, {types, icons} from './store.js';
import Await from './Await.svelte'

let promise = myfetch('/activ/categories')
</script>

<Await {promise} let:data={categories}>
	<div class="row justify-content-around m-0 p-0">
		{#each categories as cat}
			<div class="col text-center p-0 p-sm-2 {icons[cat] ? 'order-1' : 'col-12 order-2'}">
				<div class="card">
					<div class="card-header">
						<a class="card-link stretched-link text-dark text-center" href="#/{cat}" title={$types[cat].name}s> 
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
</Await>