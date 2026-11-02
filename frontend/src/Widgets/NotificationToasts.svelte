<script lang="ts">
  import { Toast } from "flowbite-svelte";
  import { toastsDoor, dismiss } from "../lib/notification.svelte";
  import * as m from "../../paraglide/messages";
</script>

<div
  class="fixed top-4 left-1/2 -translate-x-1/2 z-50 flex flex-col gap-2 w-96 max-w-[calc(100vw-2rem)]"
>
  {#each toastsDoor() as toast (toast.id)}
    <Toast
      color="red"
      align={false}
      closeAriaLabel={m.notif_dismiss()}
      class="border border-red-200 dark:border-red-800 bg-red-50 dark:bg-red-900/30 shadow-lg"
      onclose={(e: Event) => {
        e.preventDefault();
        dismiss(toast.id);
      }}
    >
      <p class="font-medium text-red-800 dark:text-red-200">
        {m.notif_title_error()}
      </p>
      <p class="mt-1 break-words text-red-700 dark:text-red-300">
        {toast.message}
      </p>
    </Toast>
  {/each}
</div>
