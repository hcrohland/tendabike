<script lang="ts">
  import { Badge } from "flowbite-svelte";
  import { CloseOutline, TrashBinOutline } from "flowbite-svelte-icons";
  import {
    history,
    unread,
    dismiss,
    readAll,
    deleteNotification,
  } from "../lib/notification.svelte";
  import type { Notification } from "../lib/notification.svelte";
  import { goto } from "$app/navigation";
  import { parts } from "../lib/part";
  import { subscriptions } from "../lib/subscription";
  import * as m from "../../paraglide/messages";
  import XsButton from "./XsButton.svelte";

  /// Closes the panel (owned by the bell's Dropdown).
  let { close } = $props();

  function title(n: Notification): string {
    switch (n.kind) {
      case "sub_request":
        return m.notif_title_sub_request();
      case "sub_response": {
        const subject = n.subject as { type: string; id: number } | null;
        if (subject?.type === "subscription" && subject.id != null) {
          const sub = subscriptions[String(subject.id)];
          if (sub?.status === "active") return m.notif_title_sub_approved();
          if (sub?.status === "rejected") return m.notif_title_sub_rejected();
        }
        return m.notif_title_sub_response();
      }
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
    // Show the notification's message when one is present: the request for a
    // request notification, the shop's reply for a response.
    if (n.message) return n.message;
    // Otherwise, for part kinds, the part's name (the subject's type
    // discriminator tells us which collection to look up).
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
      case "sub_response": {
        const subject = n.subject as { type: string; id: number } | null;
        if (subject?.type === "subscription" && subject.id != null) {
          const sub = subscriptions[String(subject.id)];
          if (sub?.status === "active") return m.notif_body_sub_approved();
          if (sub?.status === "rejected") return m.notif_body_sub_rejected();
        }
        return m.notif_body_sub_response();
      }
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

  /// The navigation target of a notification, derived from its kind:
  /// subscription kinds land on /shops (the shop-owner tab for the
  /// shop-owner kinds, the requester tab for a response); part notifications
  /// are informational (no target).
  function targetFor(n: Notification): { route: string } | null {
    switch (n.kind) {
      case "sub_request":
      case "sub_cancelled":
        return { route: "/shops?tab=my-shops" };
      case "sub_response":
        return { route: "/shops?tab=my-subscriptions" };
      case "part_registered":
      case "part_unregistered":
      default:
        return null;
    }
  }

  /// Whole-row click: navigate to the subject and mark the notification read
  /// (the same lifecycle-aware read as dismiss). A non-navigable entry does
  /// nothing.
  function navigate(n: Notification) {
    const target = targetFor(n);
    if (!target) return;
    goto(target.route);
    dismiss(n.id);
    close();
  }
</script>

<div class="flex items-center justify-between px-4 py-3">
  <h3 class="text-sm font-semibold text-gray-900 dark:text-gray-100">
    {m.notif_panel_title()}
    {#if unread() > 0}
      <Badge color="blue" class="ml-2">{unread()}</Badge>
    {/if}
  </h3>
  {#if unread() > 0}
    <XsButton
      onclick={() => {
        readAll();
        close();
      }}
    >
      {m.notif_panel_read_all()}
    </XsButton>
  {/if}
</div>
<div class="max-h-96 overflow-y-auto">
  {#if history().length === 0}
    <p class="px-4 py-8 text-center text-sm text-gray-500 dark:text-gray-400">
      {m.notif_panel_empty()}
    </p>
  {:else}
    {#each history() as n (n.id)}
      {@const navigable = targetFor(n) != null}
      <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
      <div
        class="flex items-start gap-3 px-4 py-3 border-b border-gray-100 dark:border-gray-700 last:border-b-0 hover:bg-gray-50 dark:hover:bg-gray-700/50"
        class:opacity-60={n.state === "seen" && n.lifecycle === "persistent"}
        class:cursor-pointer={navigable}
        role={navigable ? "button" : undefined}
        tabindex={navigable ? 0 : undefined}
        onclick={() => navigate(n)}
        onkeydown={(e) => {
          if (!navigable) return;
          if (e.key === "Enter" || e.key === " ") {
            e.preventDefault();
            navigate(n);
          }
        }}
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
            onclick={(e) => {
              e.stopPropagation();
              dismiss(n.id);
            }}
            aria-label={m.notif_dismiss()}
          >
            <CloseOutline class="h-4 w-4" />
          </button>
        {/if}
        {#if n.lifecycle === "persistent"}
          <button
            class="text-gray-400 hover:text-red-500 dark:hover:text-red-400 shrink-0"
            onclick={(e) => {
              e.stopPropagation();
              deleteNotification(n.id);
            }}
            aria-label={m.action_delete()}
          >
            <TrashBinOutline class="h-4 w-4" />
          </button>
        {/if}
      </div>
    {/each}
  {/if}
</div>
