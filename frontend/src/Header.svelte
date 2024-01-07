<script lang="ts">
  import {Collapse, Navbar, Nav, NavbarToggler, NavbarBrand, NavLink} from '@sveltestrap/sveltestrap';
  import {
    Dropdown,
    DropdownItem,
    DropdownMenu,
    DropdownToggle,
    NavItem,
  } from '@sveltestrap/sveltestrap';
  import {myfetch, handleError, setSummary, updateSummary, category, user} from "./store";
  import Garmin from "./Actions/Garmin.svelte"

  let userOpen = false;
  let syncOpen = false;
  let running = false;
  let number = 0;
  let promise;
  let data = undefined;
  let garmin

  function refresh () {promise = myfetch('/user/summary').then(setSummary)}

  async function getdata() {
    running = true;
    const batch = 10;
    number = 0;
    do {
      data = await myfetch('/strava/hooks').catch(handleError)
      if (!data) break;
      updateSummary(data);
      number += data["activities"].length;
    } while ($user && running && data["activities"].length > 0)
    running = false;
    number = 0;
  }

  let isOpen = false;
  function navbarUpdate(event) {
    isOpen = event.detail.isOpen;
  }

  let polling = false;
  async function poll (promiseFn, time) {
    if (polling) return
    polling = true;
    while ($user){
      promise = promiseFn()
      await promise.catch(handleError)
      await new Promise(resolve => setTimeout(resolve, time))
    }
  }

  $: if ($user) poll(getdata, 60000)

</script>

<Garmin bind:garmin />

<Navbar color="light" expand="md" >
  <img src="favicon.png" alt="TendaBike" title="TendaBike" width=60 class="rounded-circle">
  <NavbarBrand class="text-decoration-none" href="#/cat/1">
    <div class="d-md-block">
      &nbsp; Tend a Bike
    </div>
    <!-- {#if $category}
      <strong> {$category.name} </strong>
    {:else}
      Gear
    {/if} -->
  </NavbarBrand>
  <NavbarToggler on:click={() => (isOpen = !isOpen)}/>
  {#if $user}
    <Collapse {isOpen} navbar expand="md" on:update={navbarUpdate}>
      <Nav class="ms-auto float-start" navbar>
        {#if $category}
        <NavItem>
          <NavLink href="/#/cat/{$category.id}" class="dropdown-item text-reset">{$category.name}s</NavLink>
        </NavItem>
        <NavItem>
          <NavLink href="/#/spares/{$category.id}" class="dropdown-item text-reset">Spare parts</NavLink>
        </NavItem>
        <NavItem>
          <NavLink href="/#/activities" class="dropdown-item text-reset">Activities</NavLink>
        </NavItem>
        {/if}
      </Nav>
      <Nav class="ms-auto float-end" navbar>
        <Dropdown nav isOpen={syncOpen} toggle={() => (syncOpen = !syncOpen)}>
          <DropdownToggle color=light caret>
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
            <DropdownItem on:click={refresh}>Refresh data</DropdownItem>
            <DropdownItem on:click={garmin}>With CSV File</DropdownItem>
          </DropdownMenu>
        </Dropdown>
        <Dropdown nav isOpen={userOpen} toggle={() => (userOpen = !userOpen)}>
          <DropdownToggle color=light caret>{$user.firstname}</DropdownToggle>
          <DropdownMenu right>
            <DropdownItem href="/#/stats">Statistics</DropdownItem>
            <DropdownItem href="/strava/logout">Logout</DropdownItem>
            <DropdownItem divider />
            <DropdownItem href="/#/about">About</DropdownItem>
            {#if $user.is_admin}
              <DropdownItem divider />
              <DropdownItem href="/#/admin" > Admin </DropdownItem>
            {/if}
          </DropdownMenu>
        </Dropdown>
      </Nav>
    </Collapse>
  {:else}
    <Nav class="ml-auto float-end" navbar>
      <a href="/strava/login"><img src="connect_with_strava.png" alt="Login with Strava"></a>
    </Nav>
  {/if}
</Navbar>

<style>
.hamburg { 
   display: block;
   background: #fff; width: 50px; height: 40px; 
   position: relative; 
   margin-left: auto; margin-right: auto;
   border-radius: 4px; 
   transition: border-radius .5s;
}

.line { 
   position: absolute; 
   left:10px;
   height: 4px; width: 30px; 
   background: #555; border-radius: 2px;
   display: block; 
   transition: 0.5s; 
   transform-origin: center; 
}

.line:nth-child(1) { top: 10px; }
.line:nth-child(2) { top: 20px; }
.line:nth-child(3) { top: 30px; }
</style>