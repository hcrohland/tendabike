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
    DarkMode,
    Select,
  } from "flowbite-svelte";
  import { handleError, myfetch } from "./lib/store";
  import { refresh, updateSummary, user } from "./lib/user";
  import { activities } from "./lib/activity";
  import Sport from "./Widgets/Sport.svelte";
  import { category } from "./lib/types";
  import { querystring } from "svelte-spa-router";
  import { location } from "svelte-spa-router";
  import { onDestroy } from "svelte";
  import { ChevronDownOutline } from "flowbite-svelte-icons";
  import Garmin from "./Activity/Garmin.svelte";
  import ShopMenu from "./Shop/ShopMenu.svelte";
  import { shop } from "./lib/shop";
  import * as m from "./lib/paraglide/messages";
  import { getLocale, setLocale, locales } from "./lib/paraglide/runtime";

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
    hook_promise = refresh($shop?.id).then(poll);
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

  const isDev = import.meta.env.DEV;

  if (isDev) {
    const stored = localStorage.getItem("theme");
    if (stored === "dark") {
      document.documentElement.classList.add("dark");
    }
  } else {
    const mq = window.matchMedia("(prefers-color-scheme: dark)");
    document.documentElement.classList.toggle("dark", mq.matches);
    mq.addEventListener("change", (e) => {
      document.documentElement.classList.toggle("dark", e.matches);
    });
  }
</script>

<Navbar class="text-text-1">
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
          {m.header_import_activities()}
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
        <Sport />
        {#await promise then}
          <DropdownItem class="cursor-pointer flex-end">
            {m.header_sync()}
            <ChevronDownOutline class="inline" />
          </DropdownItem>
          <Dropdown simple>
            <DropdownItem onclick={fullrefresh}
              >{m.header_refresh()}</DropdownItem
            >
            <DropdownItem onclick={() => (openGarmin = true)}>
              {m.header_csv()}
            </DropdownItem>
            {#if $user.onboarding_status === "initial_sync_postponed"}
              <DropdownDivider />
              <DropdownItem onclick={triggerHistoricSync}>
                {m.header_import_historic()}
              </DropdownItem>
            {/if}
          </Dropdown>
          <Garmin bind:open={openGarmin} />
        {/await}
        <ShopMenu />
        {#if $user.is_admin}
          <DropdownDivider />
          <DropdownItem href="/#/admin">{m.header_admin()}</DropdownItem>
        {/if}
        <DropdownDivider />
        <DropdownItem href="/api/user/export" download="tendabike.json">
          {m.header_export()}
        </DropdownItem>
        <DropdownItem href="/#/about">{m.header_about()}</DropdownItem>
        <DropdownItem href="/strava/logout">{m.header_logout()}</DropdownItem>
      </Dropdown>

      <NavHamburger />
    </div>
    <NavUl
      class="max-w-full"
      {activeUrl}
      activeClass="text-primary-600 dark:text-primary-400 font-semibold"
      nonActiveClass="text-gray-700 dark:text-gray-300 hover:text-primary-600 dark:hover:text-primary-400"
    >
      <NavLi class="justify-start" href="/#/cat">{$category.name}s</NavLi>
      <NavLi href="/#/plans">{m.nav_services()}</NavLi>
      <NavLi href="/#/spares">{m.nav_parts()}</NavLi>
      {#if !$shop}
        <NavLi href="/#/activities">{m.nav_activities()}</NavLi>
        <NavLi href="/#/stats">{m.nav_statistics()}</NavLi>
      {/if}
    </NavUl>
  {:else}
    <div class="flex items-center md:order-2">
      <a href={"/strava/login?" + $querystring}>
        <img src="connect_with_strava.png" alt="Login with Strava" />
      </a>
    </div>
  {/if}
  <div class="flex">
    {#if isDev}
      <DarkMode />
      <!-- Language switcher -->
      <Select
        value={getLocale()}
        onchange={(e) => setLocale(e.currentTarget.value as any)}
        placeholder=""
      >
        {#each locales as lang}
          <option value={lang}>{lang.toUpperCase()}</option>
        {/each}
      </Select>
    {/if}
  </div>
</Navbar>
