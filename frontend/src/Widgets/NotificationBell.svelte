<script lang="ts">
  import { Badge, Dropdown } from "flowbite-svelte";
  import { BellOutline } from "flowbite-svelte-icons";
  import { unread } from "../lib/notification.svelte";
  import NotificationPanel from "./NotificationPanel.svelte";
  import * as m from "../../paraglide/messages";

  /// The panel's open state: component-local, bound to the Dropdown (the
  /// Popper's trigger is the sole toggle).
  let open = $state(false);
</script>

<div class="relative">
  <button
    id="notif-bell"
    class="relative p-2 rounded-lg hover:bg-gray-100 dark:hover:bg-gray-700 text-gray-600 dark:text-gray-300"
    aria-label={m.notif_panel_title()}
  >
    <BellOutline class="h-6 w-6" />
    {#if unread() > 0}
      <Badge color="red" rounded class="absolute -top-1 -right-1">
        {unread()}
      </Badge>
    {/if}
  </button>
  <Dropdown triggeredBy="#notif-bell" bind:isOpen={open} class="w-96">
    <NotificationPanel close={() => (open = false)} />
  </Dropdown>
</div>
