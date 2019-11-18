<script>
  import {types} from './store.js';
  import {Nav, NavItem} from 'sveltestrap';

  export let categories;
  export let category;
</script>

<style>
  .notactive {
      background-color: white;
      color: rgb(152, 152, 152);
  }
  button {
      padding-left: 2rem;
      padding-right: 2rem;
  }
  h3 {
    font-weight: inherit;
  } 
</style>

<Nav expand="md">
   <Nav class="nav-tabs">
    {#each categories as cat}
      <NavItem>
        <button class="navlink" class:notactive="{category !== cat}" on:click="{() => category = cat}">
          <h3>
            {$types[cat].name}s
          </h3>
        </button>
      </NavItem>
    {/each}
  </Nav>
</Nav>