<script lang="ts">
  import { Modal } from "flowbite-svelte";

  let { open = $bindable(), ...props } = $props();
</script>

<Modal
  form
  class="overflow-visible"
  classes={{ body: "overflow-visible" }}
  bind:open
  {...props}
/>
