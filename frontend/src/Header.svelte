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
    Avatar,
    DarkMode,
    DropdownHeader,
  } from "flowbite-svelte";
  import { user } from "./lib/store";
  import Sport from "./Widgets/Sport.svelte";
  import { category } from "./lib/types";
  import { querystring } from "svelte-spa-router";
  import SyncMenu from "./Widgets/SyncMenu.svelte";
  import { location } from "svelte-spa-router";

  let { promise } = $props();

  let activeUrl = $derived("/#" + $location);
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
  {#if $user}
    <div class="flex items-center md:order-2">
      <Avatar src={$user.avatar} id="user" class="border-2" />
      <DarkMode />
      <Dropdown simple triggeredBy="#user">
        <DropdownHeader>
          {$user.firstname}
          {$user.name}
        </DropdownHeader>
        <DropdownDivider />
        <Sport></Sport>
        {#await promise then}
          <SyncMenu></SyncMenu>
        {/await}
        {#if $user.is_admin}
          <DropdownDivider />
          <DropdownItem href="/#/admin">Admin</DropdownItem>
        {/if}
        <DropdownDivider />
        <DropdownItem href="/api/user/export" download="tendabike.json">
          Export Data
        </DropdownItem>
        <DropdownItem href="/#/about">About</DropdownItem>
        <DropdownItem href="/strava/logout">Logout</DropdownItem>
      </Dropdown>
      <NavHamburger />
    </div>
    <NavUl class="max-w-full" {activeUrl}>
      <NavLi class="justify-start" href="/#/cat">{$category.name}s</NavLi>
      <NavLi href="/#/plans">Services</NavLi>
      <NavLi href="/#/spares">Parts</NavLi>
      <NavLi href="/#/activities">Activities</NavLi>
      <NavLi href="/#/stats">Statistics</NavLi>
    </NavUl>
  {:else}
    <div class="flex items-center md:order-2">
      <a href={"/strava/login?" + $querystring}>
        <img src="connect_with_strava.png" alt="Login with Strava" />
      </a>
      <DarkMode />
    </div>
  {/if}
</Navbar>
