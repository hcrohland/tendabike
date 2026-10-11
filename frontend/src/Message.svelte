<script lang="ts">
  import { message } from "./lib/utils";
  import { Modal } from "flowbite-svelte";

  function onclose() {
    message.active = false;
  }
</script>

<Modal open={message.active} {onclose}>
  {#snippet header()}
    {message.status}
  {/snippet}

  {@html message.message}
</Modal>
