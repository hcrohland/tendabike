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
  import {
    myfetch,
    handleError,
    setSummary,
    updateSummary,
    user,
  } from "./lib/store";
  import Garmin from "./Activity/Garmin.svelte";
  import Sport from "./Widgets/Sport.svelte";
  import { category } from "./lib/types";

  let userOpen = false;
  let syncOpen = false;
  let number = 0;
  let promise: Promise<void>;
  let data = undefined;
  let garmin: () => void;

  function refresh() {
    promise = myfetch("/api/user/summary").then(setSummary);
  }

  let isOpen = false;
  function navbarUpdate(event: CustomEvent<any>) {
    isOpen = event.detail.isOpen;
  }

  async function poll() {
    try {
      if ($user) {
        do {
          data = await myfetch("/strava/hooks");
          if (!data) break;
          updateSummary(data);
          number += data["activities"].length;
        } while (data["activities"].length > 0);
        number = 0;
      }
      setTimeout(poll, 60000);
    } catch (e) {
      handleError(e as Error);
    }
  }

  poll();
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
          <NavLink href="/#/cat" class="dropdown-item text-reset"
            >{$category.name}s</NavLink
          >
        </NavItem>
        <NavItem>
          <NavLink href="/#/spares" class="dropdown-item text-reset"
            >Spare parts</NavLink
          >
        </NavItem>
        <NavItem>
          <NavLink href="/#/activities" class="dropdown-item text-reset"
            >Activities</NavLink
          >
        </NavItem>
      </Nav>
      <Nav class="ms-auto float-end" navbar>
        <Dropdown nav isOpen={syncOpen} toggle={() => (syncOpen = !syncOpen)}>
          <DropdownToggle color="light" caret>
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
      <a href="/strava/login"
        ><img src="connect_with_strava.png" alt="Login with Strava" /></a
      >
    </Nav>
  {/if}
</Navbar>
