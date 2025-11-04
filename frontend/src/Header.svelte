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
    DropdownHeader,
    Spinner,
  } from "flowbite-svelte";
  import {
    handleError,
    myfetch,
    refresh,
    updateSummary,
    user,
  } from "./lib/store";
  import { activities } from "./lib/activity";
  import Sport from "./Widgets/Sport.svelte";
  import { category } from "./lib/types";
  import { querystring } from "svelte-spa-router";
  import { location } from "svelte-spa-router";
  import { onDestroy } from "svelte";
  import { ChevronDownOutline } from "flowbite-svelte-icons";
  import Garmin from "./Activity/Garmin.svelte";

  let { promise } = $props();

  let openGarmin = $state(false);

  let hook_timer = setTimeout(() => {});

  onDestroy(() => {
    clearInterval(hook_timer);
  });

  let hook_promise = $state(poll());

  async function poll() {
    clearInterval(hook_timer);
    let data;
    try {
      do {
        data = await myfetch("/strava/hooks");
        if (!data) break;
        updateSummary(data);
      } while (data["activities"].length > 0);
      hook_timer = setTimeout(() => {
        hook_promise = poll();
      }, 60000);
    } catch (e) {
      console.error(e);
      handleError(e as Error);
    }
  }

  function fullrefresh() {
    clearInterval(hook_timer);
    hook_promise = refresh().then(poll);
  }

  async function triggerHistoricSync() {
    try {
      const updatedUser = await myfetch("/strava/onboarding/sync", "POST");
      $user = updatedUser;
      fullrefresh();
    } catch (e) {
      handleError(e as Error);
    }
  }

  let activeUrl = $derived("/#" + $location);
</script>

<Navbar>
  <NavBrand href="/#/cat">
    <img
      src="favicon.png"
      alt="TendaBike"
      title="TendaBike"
      class="rounded-circle h-11"
    />
    &nbsp; Tend a {$category.name}
  </NavBrand>
  {#if $user}
    <div class="flex items-center gap-4 md:order-2">
      {#if ($user.onboarding_status === "pending" || $user.onboarding_status === "initial_sync_postponed") && Object.keys($activities).length === 0}
        <button
          class="text-white bg-blue-700 hover:bg-blue-800 focus:ring-4 focus:ring-blue-300 font-medium rounded-lg text-sm px-4 py-2"
          onclick={triggerHistoricSync}
          type="button"
        >
          Import Activities
        </button>
      {/if}
      <div id="user">
        {#await hook_promise}
          <Spinner size="10" />
        {:then}
          <Avatar src={$user.avatar} class="border-2" />
        {:catch error}
          {handleError(error)}
        {/await}
      </div>
      <Dropdown simple triggeredBy="#user">
        <DropdownHeader>
          {$user.firstname}
          {$user.name}
        </DropdownHeader>
        <DropdownDivider />
        <Sport></Sport>
        {#await promise then}
          <DropdownItem class="cursor-pointer flex-end">
            Sync
            <ChevronDownOutline class=" inline " />
          </DropdownItem>
          <Dropdown simple>
            <DropdownItem onclick={fullrefresh}>Refresh data</DropdownItem>
            <DropdownItem onclick={() => (openGarmin = true)}>
              With CSV File
            </DropdownItem>
            {#if $user.onboarding_status === "initial_sync_postponed"}
              <DropdownDivider />
              <DropdownItem onclick={triggerHistoricSync}>
                Import Historic Activities
              </DropdownItem>
            {/if}
          </Dropdown>
          <Garmin bind:open={openGarmin} />
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
    </div>
  {/if}
</Navbar>
