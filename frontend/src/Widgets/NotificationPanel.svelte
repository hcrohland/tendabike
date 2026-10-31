<script lang="ts">
  import {
    history,
    unread,
    dismiss,
    readAll,
    isPanelOpen,
  } from "../lib/notification.svelte";
  import type { Notification } from "../lib/notification.svelte";
  import { parts } from "../lib/part";
  import * as m from "../../paraglide/messages";

  function title(n: Notification): string {
    switch (n.kind) {
      case "sub_request":
        return m.notif_title_sub_request();
      case "sub_response":
        return m.notif_title_sub_response();
      case "sub_cancelled":
        return m.notif_title_sub_cancelled();
      case "part_registered":
        return m.notif_title_part_registered();
      case "part_unregistered":
        return m.notif_title_part_unregistered();
      default:
        return m.notif_fallback();
    }
  }

  function body(n: Notification): string {
    // Use the subject's type discriminator to look up the right collection.
    const subject = n.subject as { type: string; id: number } | null;
    if (subject) {
      if (subject.type === "part" && subject.id != null) {
        const part = parts[String(subject.id)];
        if (part?.name) return part.name;
      } else if (subject.type === "subscription" && subject.id != null) {
        // Subscriptions don't have a name collection; fall through to the
        // kind-based body.
      }
    }
    // Fall back to the kind-based body
    switch (n.kind) {
      case "sub_request":
        return m.notif_body_sub_request();
      case "sub_response":
        return m.notif_body_sub_response();
      case "sub_cancelled":
        return m.notif_body_sub_cancelled();
      case "part_registered":
        return m.notif_body_part_registered();
      case "part_unregistered":
        return m.notif_body_part_unregistered();
      default:
        return m.notif_body_error();
    }
  }

  function relativeTime(dateStr: string): string {
    const date = new Date(dateStr);
    const now = new Date();
    const diffMs = now.getTime() - date.getTime();
    const minutes = Math.floor(diffMs / 60000);
    if (minutes < 1) return "now";
    if (minutes < 60) return minutes + "m";
    const hours = Math.floor(minutes / 60);
    if (hours < 24) return hours + "h";
    const days = Math.floor(hours / 24);
    return days + "d";
  }
</script>

{#if isPanelOpen()}
  <div
    class="absolute right-0 top-full mt-2 w-96 max-w-[calc(100vw-2rem)] rounded-lg border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800 shadow-lg z-40"
  >
    <div
      class="flex items-center justify-between px-4 py-3 border-b border-gray-200 dark:border-gray-700"
    >
      <h3 class="text-sm font-semibold text-gray-900 dark:text-gray-100">
        {m.notif_panel_title()}
        {#if unread() > 0}
          <span
            class="ml-2 inline-flex items-center rounded-full bg-blue-100 dark:bg-blue-900 px-2 py-0.5 text-xs font-medium text-blue-800 dark:text-blue-200"
            >{unread()}</span
          >
        {/if}
      </h3>
      {#if unread() > 0}
        <button
          class="text-xs text-blue-600 dark:text-blue-400 hover:underline"
          onclick={readAll}
        >
          {m.notif_panel_read_all()}
        </button>
      {/if}
    </div>
    <div class="max-h-96 overflow-y-auto">
      {#if history().length === 0}
        <p
          class="px-4 py-8 text-center text-sm text-gray-500 dark:text-gray-400"
        >
          {m.notif_panel_empty()}
        </p>
      {:else}
        {#each history() as n (n.id)}
          <div
            class="flex items-start gap-3 px-4 py-3 border-b border-gray-100 dark:border-gray-700 last:border-b-0 hover:bg-gray-50 dark:hover:bg-gray-700/50"
            class:opacity-60={n.state === "seen" &&
              n.lifecycle === "persistent"}
          >
            <div class="flex-1 min-w-0">
              <p
                class="text-sm font-medium text-gray-900 dark:text-gray-100"
                class:font-normal={n.state === "seen"}
              >
                {title(n)}
              </p>
              <p
                class="text-xs text-gray-500 dark:text-gray-400 mt-0.5 break-words"
              >
                {body(n)}
              </p>
              <p class="text-xs text-gray-400 dark:text-gray-500 mt-1">
                {relativeTime(n.created_at)}
              </p>
            </div>
            {#if n.state === "new"}
              <button
                class="text-gray-400 hover:text-gray-600 dark:hover:text-gray-300 shrink-0"
                onclick={() => dismiss(n.id)}
                aria-label={m.notif_dismiss()}
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
            {/if}
          </div>
        {/each}
      {/if}
    </div>
  </div>
{/if}
