<script>
  import {Collapse, NavbarToggler, NavbarBrand} from 'sveltestrap';
  import {link, push, location} from 'svelte-spa-router';
  import {myfetch, types, category, parts} from "./store.js";
  import Await from './Await.svelte';

  let disabled = false;
  let promise = undefined;

  let isOpen = false;

  async function synchronize() {
    disabled = true;
    promise = myfetch('/strava/sync?batch=100')
      .then(data => parts.updateMap(data[1]))
      .then(() => disabled = false)
  }
  function handleUpdate(event) {
    isOpen = event.detail.isOpen;
  }

</script>

<nav class="navbar navbar-expand-sm navbar-light bg-light mb-2">
  <a class="navbar-brand" href="#/">
    Tend a 
    {#if $category}
      <strong> {$category.name} </strong>
    {:else}
      Gear
    {/if}
  </a>
  <NavbarToggler on:click={() => (isOpen = !isOpen)} />
  <Collapse {isOpen} navbar expand="md" on:update={handleUpdate}>
    <ul class="navbar-nav ml-auto float-right">
      <button on:click={synchronize} {disabled} class="dropdown-item">
        <Await {promise}>
          Sync 
        </Await>
      </button>
      <a href="/about" use:link class="dropdown-item">About</a>
    </ul>
  </Collapse>
</nav>