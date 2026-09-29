<script lang="ts">
  import { DropdownItem } from "flowbite-svelte";
  import {
    changeService,
    redoService,
    deleteService,
  } from "./ServiceActions.svelte";
  import Menu from "../Widgets/Menu.svelte";
  import { m } from "../../paraglide/messages";

  let { service } = $props();
</script>

<Menu>
  <DropdownItem onclick={() => changeService(service)}>
    {m.servicemenu_change()}
  </DropdownItem>
  <DropdownItem onclick={() => redoService(service)}>
    {m.servicemenu_repeat()}
  </DropdownItem>
  <DropdownItem onclick={() => deleteService(service)}>
    {m.servicemenu_delete()}
  </DropdownItem>
</Menu>
