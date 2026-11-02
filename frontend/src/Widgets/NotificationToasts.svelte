<script lang="ts">
  import { Toast, ToastContainer } from "flowbite-svelte";
  import { toastsDoor, dismiss } from "../lib/notification.svelte";
  import * as m from "../../paraglide/messages";
</script>

<ToastContainer>
  {#each toastsDoor() as toast (toast.id)}
    <Toast
      color="red"
      align={false}
      closeAriaLabel={m.notif_dismiss()}
      onclose={(e: Event) => {
        e.preventDefault();
        dismiss(toast.id);
      }}
    >
      <p class="font-medium">{m.notif_title_error()}</p>
      <p class="mt-1 break-words">{toast.message}</p>
    </Toast>
  {/each}
</ToastContainer>
