<script lang="ts">
  import {
    Collapse,
    Navbar,
    Nav,
    NavbarToggler,
    NavbarBrand,
    NavLink,
  } from "@sveltestrap/sveltestrap";
  import {
    Dropdown,
    DropdownItem,
    DropdownMenu,
    DropdownToggle,
    NavItem,
  } from "@sveltestrap/sveltestrap";
  import { handleError, user, refresh } from "./lib/store";
  import Garmin from "./Activity/Garmin.svelte";
  import Sport from "./Widgets/Sport.svelte";
  import { category } from "./lib/types";
  import { querystring } from "svelte-spa-router";

  let userOpen = false;
  let syncOpen = false;
  let promise: Promise<void>;
  let garmin: () => void;

  let isOpen = false;
  function navbarUpdate(event: CustomEvent<any>) {
    isOpen = event.detail.isOpen;
  }
</script>

<Garmin bind:garmin />

<Navbar color="light" expand="md">
  <img
    src="favicon.png"
    alt="TendaBike"
    title="TendaBike"
    width="60"
    class="rounded-circle"
  />
  <NavbarBrand class="text-decoration-none" href="/#/cat">
    &nbsp; Tend a {$category.name}
  </NavbarBrand>
  <NavbarToggler on:click={() => (isOpen = !isOpen)} />
  {#if $user}
    <Collapse {isOpen} navbar expand="md" on:update={navbarUpdate}>
      <Nav class="ms-auto float-start" navbar>
        <NavItem>
          <NavLink href="/#/cat" class="dropdown-item text-reset">
            {$category.name}s
          </NavLink>
        </NavItem>
        <NavItem>
          <NavLink href="/#/plans" class="dropdown-item text-reset">
            Services
          </NavLink>
        </NavItem>
        <NavItem>
          <NavLink href="/#/spares" class="dropdown-item text-reset">
            Parts
          </NavLink>
        </NavItem>
        <NavItem>
          <NavLink href="/#/activities" class="dropdown-item text-reset">
            Activities
          </NavLink>
        </NavItem>
      </Nav>
      <Nav class="ms-auto float-end" navbar>
        <Dropdown nav isOpen={syncOpen} toggle={() => (syncOpen = !syncOpen)}>
          <DropdownToggle color="light" caret>
            {#await promise}
              Syncing ...
            {:then}
              Sync
            {:catch error}
              {handleError(error)}
            {/await}
          </DropdownToggle>
          <DropdownMenu right>
            <DropdownItem on:click={() => (promise = refresh())}
              >Refresh data
            </DropdownItem>
            <DropdownItem on:click={garmin}>With CSV File</DropdownItem>
          </DropdownMenu>
        </Dropdown>
        <Dropdown nav isOpen={userOpen} toggle={() => (userOpen = !userOpen)}>
          <DropdownToggle color="light" caret>{$user.firstname}</DropdownToggle>
          <DropdownMenu right>
            <DropdownItem href="/#/stats">Statistics</DropdownItem>
            <Sport></Sport>
            <DropdownItem href="/strava/logout">Logout</DropdownItem>
            <DropdownItem divider />
            <DropdownItem href="/#/about">About</DropdownItem>
            {#if $user.is_admin}
              <DropdownItem divider />
              <DropdownItem href="/#/admin">Admin</DropdownItem>
            {/if}
          </DropdownMenu>
        </Dropdown>
      </Nav>
    </Collapse>
  {:else}
    <Nav class="ml-auto float-end" navbar>
      <a href={"/strava/login?" + $querystring}>
        <img src="connect_with_strava.png" alt="Login with Strava" />
      </a>
    </Nav>
  {/if}
</Navbar>
