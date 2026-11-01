<script lang="ts">
  import {
    unread,
    isPanelOpen,
    setPanelOpen,
  } from "../lib/notification.svelte";
  import NotificationPanel from "./NotificationPanel.svelte";
  import * as m from "../../paraglide/messages";

  /// The bell + panel wrapper; a click landing outside it closes the panel.
  let container = $state<HTMLElement | null>(null);

  $effect(() => {
    if (!isPanelOpen()) return;
    function onClickOutside(e: MouseEvent) {
      if (container && !container.contains(e.target as Node)) {
        setPanelOpen(false);
      }
    }
    document.addEventListener("click", onClickOutside);
    return () => document.removeEventListener("click", onClickOutside);
  });
</script>

<div class="relative" bind:this={container}>
  <button
    class="relative p-2 rounded-lg hover:bg-gray-100 dark:hover:bg-gray-700 text-gray-600 dark:text-gray-300"
    onclick={() => setPanelOpen(!isPanelOpen())}
    aria-label={m.notif_panel_title()}
  >
    <svg
      class="h-6 w-6"
      fill="none"
      viewBox="0 0 24 24"
      stroke="currentColor"
      stroke-width="1.5"
    >
      <path
        stroke-linecap="round"
        stroke-linejoin="round"
        d="M14.857 17.082a23.848 23.848 0 005.454-1.31A8.967 8.967 0 0118 9.75v-.7V9A6 6 0 006 9v.75a8.967 8.967 0 01-2.312 6.022c1.733.64 3.56 1.085 5.455 1.31m5.714 0a24.255 24.255 0 01-5.714 0m5.714 0a3 3 0 11-5.714 0"
      />
    </svg>
    {#if unread() > 0}
      <span
        class="absolute -top-1 -right-1 inline-flex items-center justify-center h-5 min-w-5 rounded-full bg-red-500 text-white text-xs font-medium px-1"
      >
        {unread()}
      </span>
    {/if}
  </button>
  <NotificationPanel />
</div>
