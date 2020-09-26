<script>
  import {Collapse, NavbarToggler, NavbarBrand} from 'sveltestrap';
  import {link, push, location} from 'svelte-spa-router';
  import {myfetch, handleError, updatePartAttach, category, user} from "./store.js";

  let running = false;
  let number = 0;
  let promise = getdata();
  let data = undefined;

  let isOpen = false;

  function synchronize () {
    if (running) {
      running = false
    } else {
      promise = getdata();
    } 
  }

  async function getdata() {
    running = true;
    const batch = 10;
    number = batch;
    do {
      data = await myfetch('/strava/sync?batch=' + batch)
      updatePartAttach(data[1]);
      number += data[0].length;
    } while (running && data[0].length == batch)
    running = false;
  }
  function handleUpdate(event) {
    isOpen = event.detail.isOpen;
  }

</script>

<nav class="navbar navbar-expand-md navbar-light bg-light mb-2">
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
    <ul class="navbar-nav ml-auto float-left">
      {#if $category}
         <a href="/cat/{$category.id}" use:link class="dropdown-item text-reset">{$category.name}s</a>
         <a href="/spares/{$category.id}" use:link class="dropdown-item text-reset">Spare parts</a>
      {/if}
    </ul>
    <ul class="navbar-nav ml-auto float-right">
      <button on:click={synchronize} class="dropdown-item">
        {#await promise}
          Syncing {number}...
        {:then value}
          Sync 
        {:catch error}
          {handleError(error)}
        {/await}
      </button>
      <a href="/about" use:link class="dropdown-item text-reset">About</a>
      {#if $user}
      <a href="/user" use:link class="dropdown-item text-reset">
        {$user.firstname}
      </a> 
      {:else}
         <!-- else content here -->
      {/if}
    </ul>
  </Collapse>
</nav>