<script>
  import {types} from './store.js';

  export let categories;
  export let category;
</script>

<style>
  h3 {
    font-weight: inherit;
  } 
</style>

<ul class="nav nav-tabs">
  {#each categories as cat}
    <li class="nav-item">
      <button class="btn btn-secondary navlink px-2 px-sm-4 {category === cat ? 'active' : ''}" 
              on:click="{() => category = cat}">
        <h3>
          {$types[cat].name}s
        </h3>
      </button>
    </li>
  {/each}
</ul>