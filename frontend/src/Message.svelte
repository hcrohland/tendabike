<script lang="ts">
  import { message } from "./lib/store";
  import { Modal } from "flowbite-svelte";

  function onclose() {
    let m = $message;
    m.active = false;
    message.set(m);
  }
</script>

<Modal open={$message.active} {onclose}>
  {#snippet header()}
    {$message.status}
  {/snippet}

  {@html $message.message}
</Modal>
