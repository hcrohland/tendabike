<script lang="ts">
  import { DropdownItem, Spinner } from "flowbite-svelte";
  import Card from "../Widgets/Card.svelte";
  import Chip from "../Widgets/Chip.svelte";
  import Menu from "../Widgets/Menu.svelte";
  import { handleError, myfetch } from "../lib/utils";
  import type { User } from "../lib/user";
  import * as m from "../../paraglide/messages";

  let {
    user,
    parts,
    activities,
    events,
    disabled = false,
    refresh,
    createSync,
    deleteUser,
  }: {
    user: User;
    parts: number;
    activities: number;
    events: number;
    disabled?: boolean;
    refresh: () => void;
    createSync: (user: User) => void;
    deleteUser: (user: User) => void;
  } = $props();

  let syncing = $state(false);

  let role = $derived(
    disabled
      ? m.admin_role_disabled()
      : user.is_admin
        ? m.header_admin()
        : m.admin_role_user(),
  );

  async function sync() {
    syncing = true;
    // the backend queues the activities onto the per-user executor; the new
    // rows arrive over the stream, so the client just awaits the write
    await myfetch("/strava/sync/" + user.id).catch(handleError);
    refresh();
    syncing = false;
  }

  async function disable() {
    await myfetch("/strava/disable/" + user.id, "POST").catch(handleError);
    refresh();
  }
</script>

<Card>
  <!-- Header row: name · role · menu -->
  <div class="flex items-center justify-between gap-1 sm:gap-2">
    <div class="flex items-center gap-1 sm:gap-2 p-1 sm:p-2 min-w-0">
      <span class="font-medium text-sm whitespace-nowrap">
        {user.firstname}
        {user.name}
      </span>
      <span class="text-xs text-text-1 shrink-0">{role}</span>
    </div>
    <div class="flex items-center gap-1 sm:gap-2 shrink-0">
      {#if syncing}
        <Spinner size="16" />
      {:else}
        <Menu>
          {#if !disabled}
            <DropdownItem onclick={() => createSync(user)}>
              {m.admin_add_sync_event()}
            </DropdownItem>
            <DropdownItem onclick={sync}>{m.sync_process_queue()}</DropdownItem>
            <DropdownItem onclick={disable}>
              {m.admin_disable_user()}
            </DropdownItem>
          {/if}
          <DropdownItem onclick={() => deleteUser(user)}>
            {m.admin_delete_user()}
          </DropdownItem>
        </Menu>
      {/if}
    </div>
  </div>

  <!-- Counts -->
  <div class="flex flex-wrap items-center gap-1 sm:gap-2">
    <Chip value={String(parts)} label={m.nav_parts()} />
    <Chip value={String(activities)} label={m.nav_activities()} />
    <Chip value={String(events)} label={m.admin_col_events()} />
  </div>
</Card>
