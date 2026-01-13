<script lang="ts">
  import {
    DropdownDivider,
    DropdownItem,
    Label,
    Select,
  } from "flowbite-svelte";
  import { Shop, shop, shops } from "../lib/shop";
  import { user } from "../lib/user";
  import type { Part } from "../lib/part";
  import type { Attachment } from "../lib/attachment";

  interface Props {
    part: Part;
    last_attachment?: Attachment;
  }

  let { part, last_attachment }: Props = $props();

  let disabled = $derived(
    last_attachment && last_attachment.isAttached() && !part.isGear(),
  );

  // Fetch user's shops (only owned shops, not in shop mode)
  let userShops = $derived($shop ? [] : Object.values($shops));

  async function unregisterFromShop() {
    try {
      await Shop.unregisterPart(part);
    } catch (error) {
      console.error("Error unregistering part:", error);
    }
  }

  // Toggle registration of this part to a shop
  async function registerToShop(shopid: number) {
    try {
      await Shop.registerPart(part, shopid);
    } catch (error) {
      console.error("Error registering to shop:", error);
    }
  }
</script>

{#if part.shop || !disabled}
  {#if part.shop && !disabled}
    <DropdownDivider />
    <DropdownItem onclick={unregisterFromShop}>
      Move out of {$shops[part.shop]?.name}
    </DropdownItem>
  {:else if !$shop && $user?.id === part.owner && userShops.length > 0}
    <DropdownDivider />
    <DropdownItem class="flex items-center gap-2">
      <Label>
        Put into
        <Select
          onchange={(e: any) => registerToShop(e.target.value)}
          {disabled}
          placeholder="Choose shop..."
        >
          {#each userShops as shop}
            <option value={shop.id}>
              {shop.name}
            </option>
          {/each}
        </Select>
      </Label>
    </DropdownItem>
  {/if}
{/if}
