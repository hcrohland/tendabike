<script lang="ts">
  import {
    Clipboard,
    Dropdown,
    DropdownItem,
    Input,
    Tooltip,
  } from "flowbite-svelte";
  import { CheckOutline, ClipboardCleanSolid } from "flowbite-svelte-icons";
  import * as m from "../../paraglide/messages";
  import { getActions } from "../Widgets/Actions.svelte";
  import Menu from "../Widgets/Menu.svelte";

  let { shop } = $props();
</script>

<Menu>
  <DropdownItem onclick={() => getActions()!.editShop(shop)}>
    {m.shop_edit()}
  </DropdownItem>

  <DropdownItem onclick={() => getActions()!.deleteShop(shop)}>
    {m.shop_delete()}
  </DropdownItem>

  <DropdownItem>
    {m.shop_registration_link()}
  </DropdownItem>

  <Dropdown simple>
    <DropdownItem>
      {@const value = window.location.origin + "/#/register/" + shop.id}

      <Input {value} readonly>
        {#snippet right()}
          <Clipboard {value} embedded>
            {#snippet children(success)}
              <Tooltip isOpen={success}>
                {success
                  ? m.shop_link_copied()
                  : m.shop_copy_registration_link()}
              </Tooltip>

              {#if success}
                <CheckOutline />
              {:else}
                <ClipboardCleanSolid />
              {/if}
            {/snippet}
          </Clipboard>
        {/snippet}
      </Input>
    </DropdownItem>
  </Dropdown>
</Menu>
