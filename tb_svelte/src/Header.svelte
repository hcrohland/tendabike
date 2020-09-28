<script>
  import {Collapse, NavbarToggler, NavbarBrand} from 'sveltestrap';
  import {
    Dropdown,
    DropdownItem,
    DropdownMenu,
    DropdownToggle
  } from 'sveltestrap';

  let menuOpen = false;

  import {link, push, location} from 'svelte-spa-router';
  import {myfetch, handleError, updatePartAttach, category, user} from "./store.js";

  let running = false;
  let number = 0;
  let promise;
  let data = undefined;

  let isOpen = false;

  $: if ($user) {promise = getdata()}

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
    number = 0;
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
  {#if $user}
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
            {#if number}
              Synced {number} ...
            {:else}
              Syncing...
            {/if}
          {:then}
            Sync 
          {:catch error}
            {handleError(error)}
          {/await}
        </button>
        <Dropdown navBar isOpen={menuOpen} toggle={() => (menuOpen = !menuOpen)}>
          <DropdownToggle nav caret>{$user.firstname}</DropdownToggle>
          <DropdownMenu right>
            <DropdownItem><a href="/strava/logout" class="btn text-reset">Logout</a></DropdownItem>
            <DropdownItem divider />
            <DropdownItem><a href="/about" use:link class="btn text-reset">About</a></DropdownItem>
          </DropdownMenu>
        </Dropdown>
      </ul>
    </Collapse>
  {:else}
    <ul class="navbar-nav ml-auto float-right">
      <a href="/strava/login">Login with Strava</a>
    </ul>
  {/if}
</nav>