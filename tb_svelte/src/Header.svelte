<script>
  import {Collapse, Navbar, Nav, NavbarToggler, NavbarBrand} from 'sveltestrap';
  import {
    Dropdown,
    DropdownItem,
    DropdownMenu,
    DropdownToggle
  } from 'sveltestrap';

  let userOpen = false;
  let syncOpen = false;

  import {link, push, location} from 'svelte-spa-router';
  import {myfetch, handleError, setPartAttach, updatePartAttach, category, user} from "./store.js";
  import Garmin from "./Garmin.svelte"

  let running = false;
  let number = 0;
  let promise;
  let data = undefined;

  let isOpen = false;

  $: if ($user) {promise = getdata()}

  function refresh () {promise = myfetch('/part/all').then(setPartAttach)}
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
      data = await myfetch('/strava/hooks')
      updatePartAttach(data);
      number += data["activities"].length;
    } while (running && data["activities"].length > 0)
    running = false;
    number = 0;
  }
  function handleUpdate(event) {
    isOpen = event.detail.isOpen;
  }
  let garmin
</script>

<Garmin bind:toggle={garmin} />

<Navbar expand="md" color="light" light mb-2>
  <img src="favicon.png" alt="TendaBike" title="TendaBike" width=60 class="rounded-circle">
  <NavbarBrand href="#/cat/1">
    <div class="d-none d-md-block">
      &nbsp; Tend a Bike
    </div>
    <!-- {#if $category}
      <strong> {$category.name} </strong>
    {:else}
      Gear
    {/if} -->
  </NavbarBrand>
  {#if $user}
    <NavbarToggler on:click={() => (isOpen = !isOpen)} />
    <Collapse {isOpen} navbar expand="md" on:update={handleUpdate}>
      <Nav class="ml-auto float-left" navbar>
        {#if $category}
          <a href="/cat/{$category.id}" use:link class="dropdown-item text-reset">{$category.name}s</a>
          <a href="/spares/{$category.id}" use:link class="dropdown-item text-reset">Spare parts</a>
        {/if}
      </Nav>
      <Nav class="ml-auto float-right" navbar>
        <Dropdown nav isOpen={syncOpen} toggle={() => (syncOpen = !syncOpen)}>
          <DropdownToggle nav caret>
            {#await promise}
              Syncing
              {#if number != 0}
                {number}
              {:else}
                ...
              {/if}
            {:then}
              Sync 
            {:catch error}
              {handleError(error)}
            {/await}
          </DropdownToggle>
          <DropdownMenu right>
            <DropdownItem on:click={synchronize}>
              {#if running}
                Stop Syncing
              {:else}
                Strava
              {/if}
            </DropdownItem>
            <DropdownItem on:click={garmin}>Garmin</DropdownItem>
            <DropdownItem on:click={refresh}>Refresh</DropdownItem>
          </DropdownMenu>
        </Dropdown>
        <Dropdown nav isOpen={userOpen} toggle={() => (userOpen = !userOpen)}>
          <DropdownToggle nav caret>{$user.firstname}</DropdownToggle>
          <DropdownMenu right>
            <DropdownItem href="/strava/logout">Logout</DropdownItem>
            <DropdownItem divider />
            <DropdownItem href="/#/about">About</DropdownItem>
          </DropdownMenu>
        </Dropdown>
      </Nav>
    </Collapse>
  {:else}
    <Nav class="ml-auto float-right" navbar>
      <a href="/strava/login"><img src="connect_with_strava.png" alt="Login with Strava"></a>
    </Nav>
  {/if}
  </Navbar>