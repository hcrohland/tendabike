<script lang="ts">
  import {Collapse, Navbar, Nav, NavbarToggler, NavbarBrand} from 'sveltestrap';
  import {
    Dropdown,
    DropdownItem,
    DropdownMenu,
    DropdownToggle
  } from 'sveltestrap';
  import {link} from 'svelte-spa-router';
  import {myfetch, handleError, setPartAttach, updatePartAttach, category, user} from "./store";
  import Garmin from "./Actions/Garmin.svelte"

  let userOpen = false;
  let syncOpen = false;
  let running = false;
  let polling = false;
  let number = 0;
  let promise;
  let data = undefined;
  let garmin
  let isOpen = false;

  $: if ($user && !polling) {
    polling = true;
    poll(() => new Promise(() => {promise = getdata()}), 60000)
  }

  function refresh () {promise = myfetch('/part/all').then(setPartAttach)}

  async function getdata() {
    running = true;
    const batch = 10;
    number = 0;
    do {
      data = await myfetch('/strava/hooks').catch((err) => { console.log(err);})
      if (!data) break;
      updatePartAttach(data);
      number += data["activities"].length;
    } while (running && data["activities"].length > 0)
    running = false;
    number = 0;
  }
  function handleUpdate(event) {
    isOpen = event.detail.isOpen;
  }

  var sleep = time => new Promise(resolve => setTimeout(resolve, time))
  export var poll = (promiseFn, time) => promiseFn().then(
              sleep(time).then(() => poll(promiseFn, time)))


</script>

<Garmin bind:garmin />

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
            <DropdownItem on:click={garmin}>Garmin</DropdownItem>
          </DropdownMenu>
        </Dropdown>
        <Dropdown nav isOpen={userOpen} toggle={() => (userOpen = !userOpen)}>
          <DropdownToggle nav caret>{$user.firstname}</DropdownToggle>
          <DropdownMenu right>
            <DropdownItem on:click={refresh}>Refresh</DropdownItem>
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