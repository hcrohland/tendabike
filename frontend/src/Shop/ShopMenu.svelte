<script lang="ts">
  import { Dropdown, DropdownDivider, DropdownItem } from "flowbite-svelte";
  import { ChevronDownOutline } from "flowbite-svelte-icons";
  import * as m from "../../paraglide/messages";

  import { Shop, shops, getShop, setShop } from "../lib/shop";
  import { refresh, getUser } from "../lib/user";
  import { stateValues } from "../lib/mapable.svelte";

  let myshops = $derived(stateValues(shops));

  // Enter shop mode: replaces stores with shop-specific data
  async function enterShop(myshop: Shop) {
    setShop(myshop);
    if (myshop.owner == getUser()?.id) await refresh(myshop.id);

    // Navigate to main page
    window.location.hash = "#/cat";
  }
</script>

{#if !getShop()}
  <DropdownDivider />

  {#if myshops.length == 1}
    <DropdownItem onclick={() => enterShop(myshops[0])}>
      {m.shop_enter_named({ name: myshops[0].name })}
    </DropdownItem>
  {:else if myshops.length > 1}
    <DropdownItem class="cursor-pointer flex-end">
      {m.shop_enter()}
      <ChevronDownOutline class="inline" />
    </DropdownItem>

    <Dropdown simple>
      {#each myshops as shop}
        <DropdownItem onclick={() => enterShop(shop)}>
          {shop.name}
        </DropdownItem>
      {/each}
    </Dropdown>
  {/if}

  <DropdownItem href="/#/shops">
    {m.shop_manage()}
  </DropdownItem>
{/if}
