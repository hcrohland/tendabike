<script>
  import {types} from './store.js';

  export let categories;
  export let category;
</script>

<style>
  .notactive {
      background-color: white;
      color: rgb(152, 152, 152);
  }

  .notactive:hover {
    background-color: lightgray;
  }
  h3 {
    font-weight: inherit;
    color: primary;
  } 
</style>

<ul class="nav nav-tabs">
  {#each categories as cat}
    <li class="nav-item">
      <button class="navlink mr-0 mr-sm-2 mr-md-4 px-2 px-sm-4" class:notactive="{category !== cat}" on:click="{() => category = cat}">
        <h3>
          {$types[cat].name}s
        </h3>
      </button>
    </li>
  {/each}
</ul>