<script>
import myfetch, {types, category} from './store.js';

let promise = myfetch('/part/categories')
$category = null;
</script>

{#await promise}
	loading
{:then data}
	{#each data as cat}
		 <li>
		 <a href="#/{cat}"> {$types[cat].name} </a>
		 </li>
	{/each}
{:catch error}
  {error}
{/await}
