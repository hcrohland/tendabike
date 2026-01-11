<script lang="ts">
  import { Dropdown, DropdownDivider, DropdownItem } from "flowbite-svelte";
  import { Shop, shops } from "../lib/shop";
  import { refresh, user } from "../lib/store";
  import { shop } from "../lib/shop";
  import { ChevronDownOutline } from "flowbite-svelte-icons";

  let myshops = $derived(Object.values($shops));

  // Enter shop mode: replaces stores with shop-specific data
  async function enterShop(myshop: Shop) {
    shop.set(myshop);
    if (myshop.owner == $user?.id) await refresh(myshop.id);

    // Navigate to main page
    window.location.hash = "#/cat";
  }
</script>

{#if !$shop}
  <DropdownDivider />
  {#if myshops.length == 1}
    <DropdownItem onclick={() => enterShop(myshops[0])}>
      Enter {myshops[0].name}
    </DropdownItem>
  {:else if myshops.length > 1}
    <DropdownItem class="cursor-pointer flex-end">
      Enter shop
      <ChevronDownOutline class=" inline " />
    </DropdownItem>
    <Dropdown simple>
      {#each myshops as shop}
        <DropdownItem onclick={() => enterShop(shop)}>
          {shop.name}
        </DropdownItem>
      {/each}
    </Dropdown>
  {/if}
  <DropdownItem href="/#/shops">Manage shops</DropdownItem>
{/if}
