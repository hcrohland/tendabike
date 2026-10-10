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
  import { refresh, getUser, setUser } from "./lib/user";
  import { markHydrated, startStream, stopStream } from "./lib/stream";
  import { activities } from "./lib/activity";
  import { stateValues } from "./lib/mapable.svelte";
  import Sport from "./Widgets/Sport.svelte";
  import { getCategory } from "./lib/types";
  import { querystring } from "svelte-spa-router";
  import { location } from "svelte-spa-router";
  import { ChevronDownOutline } from "flowbite-svelte-icons";
  import Garmin from "./Activity/Garmin.svelte";
  import ShopMenu from "./Shop/ShopMenu.svelte";
  import { getShop } from "./lib/shop";
  import * as m from "../paraglide/messages";
  import { getLocale, setLocale, locales } from "../paraglide/runtime";
  import { onDestroy } from "svelte";

  let { promise } = $props();

  let openGarmin = $state(false);

  /// The avatar spinner tracks the initial catch-up snapshot (part of
  /// initData's promise) and every reconnect snapshot.
  // svelte-ignore state_referenced_locally — `promise` is the initData
  // promise created once in App's module scope; its identity is stable, so
  // capturing it at init is the intent (it is the initial snapshot).
  let hook_promise = $state(promise);

  /// The stream opens only once the user is known: an anonymous session
  /// gets the About page (the 401 redirect), never a stream. The initial
  /// snapshot (initData's refresh) is the single hydration; the stream
  /// buffers the frames that race it until it settles — a failed hydration
  /// flushes too: the frames are the newest state, and the buffer must not
  /// grow unbounded.
  $effect(() => {
    promise.then(
      () => markHydrated(),
      () => markHydrated(),
    );
    if (getUser()) {
      startStream((p) => (hook_promise = p));
    }
  });

  onDestroy(() => stopStream());

  function fullrefresh() {
    hook_promise = refresh(getShop()?.id);
  }

  async function triggerHistoricSync() {
    try {
      const updatedUser = await myfetch("/strava/onboarding/sync", "POST");
      setUser(updatedUser);
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
    &nbsp; Tend a {getCategory()!.name}
  </NavBrand>
  {#if getUser()}
    <div class="flex items-center gap-4 md:order-2">
      {#if (getUser()?.onboarding_status === "pending" || getUser()?.onboarding_status === "initial_sync_postponed") && stateValues(activities).length === 0}
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
          <Avatar src={getUser()?.avatar} class="border-2" />
        {:catch error}
          {handleError(error)}
        {/await}
      </div>

      <Dropdown simple triggeredBy="#user">
        <DropdownHeader>
          {getUser()?.firstname}
          {getUser()?.name}
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
            {#if getUser()?.onboarding_status === "initial_sync_postponed"}
              <DropdownDivider />
              <DropdownItem onclick={triggerHistoricSync}>
                {m.header_import_historic()}
              </DropdownItem>
            {/if}
          </Dropdown>
          <Garmin bind:open={openGarmin} />
        {/await}
        <ShopMenu />
        {#if getUser()?.is_admin}
          <DropdownDivider />
          <DropdownItem href="/#/admin">{m.header_admin()}</DropdownItem>
        {/if}
        <DropdownDivider />
        <DropdownItem href="/api/user/export" download="tendabike.json">
          {m.header_export()}
        </DropdownItem>

        <DropdownItem href="/#/about">{m.header_about()}</DropdownItem>
        <DropdownItem href="/strava/logout">{m.header_logout()}</DropdownItem>
        <DropdownItem>
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
        </DropdownItem>
        {#if isDev}
          <DropdownItem>
            <DarkMode />
          </DropdownItem>
        {/if}
      </Dropdown>

      <NavHamburger />
    </div>
    <NavUl
      class="max-w-full"
      {activeUrl}
      classes={{
        active: "text-primary-600 dark:text-primary-400 font-semibold",
        nonActive:
          "text-gray-700 dark:text-gray-300 hover:text-primary-600 dark:hover:text-primary-400",
      }}
    >
      <NavLi class="justify-start" href="/#/cat">
        {getCategory()!.localizedName()}s
      </NavLi>
      <NavLi href="/#/plans">{m.nav_services()}</NavLi>
      <NavLi href="/#/spares">{m.nav_parts()}</NavLi>
      {#if !getShop()}
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
</Navbar>
