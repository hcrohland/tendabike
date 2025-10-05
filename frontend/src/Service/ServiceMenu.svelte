<script lang="ts">
  import { DropdownItem } from "flowbite-svelte";
  import { actions } from "../Widgets/Actions.svelte";
  import Menu from "../Widgets/Menu.svelte";

  let { service } = $props();
</script>

<Menu>
  <DropdownItem onclick={() => $actions.changeService(service)}>
    Change Service
  </DropdownItem>
  <DropdownItem onclick={() => $actions.repeatService(service)}>
    Repeat Service
  </DropdownItem>
  <DropdownItem onclick={() => $actions.deleteService(service)}>
    Delete Service
  </DropdownItem>
</Menu>
