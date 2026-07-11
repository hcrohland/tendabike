<script lang="ts">
  import { DropdownItem } from "flowbite-svelte";
  import { actions } from "../Widgets/Actions.svelte";
  import Menu from "../Widgets/Menu.svelte";
  import { m } from "../../paraglide/messages";

  let { service } = $props();
</script>

<Menu>
  <DropdownItem onclick={() => $actions.changeService(service)}>
    {m.servicemenu_change()}
  </DropdownItem>
  <DropdownItem onclick={() => $actions.redoService(service)}>
    {m.servicemenu_repeat()}
  </DropdownItem>
  <DropdownItem onclick={() => $actions.deleteService(service)}>
    {m.servicemenu_delete()}
  </DropdownItem>
</Menu>
