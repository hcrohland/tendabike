<script lang="ts">
  import { toastsDoor, dismiss } from "../lib/notification.svelte";
  import * as m from "../../paraglide/messages";
</script>

<div
  class="fixed top-4 left-1/2 -translate-x-1/2 z-50 flex flex-col gap-2 w-96 max-w-[calc(100vw-2rem)]"
>
  {#each toastsDoor() as toast (toast.id)}
    <div
      class="flex items-start gap-3 rounded-lg bg-red-50 dark:bg-red-900/30 border border-red-200 dark:border-red-800 p-4 shadow-lg"
    >
      <div class="flex-1 min-w-0">
        <p class="text-sm font-medium text-red-800 dark:text-red-200">
          {m.notif_title_error()}
        </p>
        <p class="text-sm text-red-700 dark:text-red-300 mt-1 break-words">
          {toast.message}
        </p>
      </div>
      <button
        class="text-red-500 hover:text-red-700 dark:text-red-400 dark:hover:text-red-200"
        onclick={() => dismiss(toast.id)}
        aria-label="Dismiss"
      >
        <svg
          class="h-4 w-4"
          fill="none"
          viewBox="0 0 24 24"
          stroke="currentColor"
          stroke-width="2"
        >
          <path
            stroke-linecap="round"
            stroke-linejoin="round"
            d="M6 18L18 6M6 6l12 12"
          />
        </svg>
      </button>
    </div>
  {/each}
</div>
