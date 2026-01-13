<script lang="ts">
  import {
    Clipboard,
    Dropdown,
    DropdownItem,
    Input,
    Tooltip,
  } from "flowbite-svelte";
  import {
    CheckOutline,
    ClipboardCleanSolid,
    DotsVerticalOutline,
  } from "flowbite-svelte-icons";
  import { actions } from "../Widgets/Actions.svelte";

  let { shop } = $props();
</script>

<DotsVerticalOutline class="cursor-pointer" />
<Dropdown>
  <DropdownItem onclick={() => $actions.editShop(shop)}>Edit Shop</DropdownItem>
  <DropdownItem onclick={() => $actions.deleteShop(shop)}>
    Delete Shop
  </DropdownItem>
  <DropdownItem>Registration Link</DropdownItem>
  <Dropdown simple>
    <DropdownItem>
      {@const value = window.location.origin + "/#/register/" + shop.id}
      <Input {value} readonly>
        {#snippet right()}
          <Clipboard {value} embedded>
            {#snippet children(success)}
              <Tooltip isOpen={success}>
                {success ? "Copied" : "Copy to clipboard"}
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
</Dropdown>
