<script lang="ts">
  import { DropdownItem } from "flowbite-svelte";
  import { getActions } from "../Widgets/Actions.svelte";
  import Menu from "../Widgets/Menu.svelte";
  import { m } from "../../paraglide/messages";

  let { service } = $props();
</script>

<Menu>
  <DropdownItem onclick={() => getActions()!.changeService(service)}>
    {m.servicemenu_change()}
  </DropdownItem>
  <DropdownItem onclick={() => getActions()!.redoService(service)}>
    {m.servicemenu_repeat()}
  </DropdownItem>
  <DropdownItem onclick={() => getActions()!.deleteService(service)}>
    {m.servicemenu_delete()}
  </DropdownItem>
</Menu>
