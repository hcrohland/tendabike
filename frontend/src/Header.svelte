<script lang="ts">
  import {
    Navbar,
    NavBrand,
    NavLi,
    NavUl,
    NavHamburger,
    Dropdown,
    DropdownItem,
    DropdownDivider,
  } from "flowbite-svelte";
  import { ChevronDownOutline } from "flowbite-svelte-icons";
  import { user } from "./lib/store";
  import Sport from "./Widgets/Sport.svelte";
  import { category } from "./lib/types";
  import { querystring } from "svelte-spa-router";
  import SyncMenu from "./Widgets/SyncMenu.svelte";

  let { promise } = $props();
</script>

<Navbar>
  <NavBrand href="/#/cat">
    <img
      src="favicon.png"
      alt="TendaBike"
      title="TendaBike"
      width="60"
      class="rounded-circle"
    />
    &nbsp; Tend a {$category.name}
  </NavBrand>
  <div class="flex items-center md:order-2">
    {#if $user}
      <button id="user">
        {$user.firstname}
        <ChevronDownOutline class=" inline " />
      </button>

      <Dropdown simple triggeredBy="#user">
        <DropdownItem href="/#/stats">Statistics</DropdownItem>
        <Sport></Sport>
        <DropdownItem href="/strava/logout">Logout</DropdownItem>
        <DropdownItem href="/api/user/export" download="tendabike.json">
          Export Data
        </DropdownItem>
        <DropdownDivider />
        <DropdownItem href="/#/about">About</DropdownItem>
        {#if $user.is_admin}
          <DropdownDivider />
          <DropdownItem href="/#/admin">Admin</DropdownItem>
        {/if}
      </Dropdown>
    {/if}
    <NavHamburger />
  </div>
  {#if $user}
    <NavUl class="max-w-full">
      <NavLi class="justify-start" href="/#/cat">{$category.name}s</NavLi>
      <NavLi href="/#/plans">Services</NavLi>
      <NavLi href="/#/spares">Parts</NavLi>
      <NavLi href="/#/activities">Activities</NavLi>
      {#await promise then}
        <SyncMenu></SyncMenu>
      {/await}
    </NavUl>
  {:else}
    <NavUl>
      <NavLi href={"/strava/login?" + $querystring}>
        <img src="connect_with_strava.png" alt="Login with Strava" />
      </NavLi>
    </NavUl>
  {/if}
</Navbar>
