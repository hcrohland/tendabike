<script lang="ts">
  import { Dropdown, DropdownDivider, DropdownItem } from "flowbite-svelte";
  import { shops } from "../lib/shop";
  import { user } from "../lib/store";
  import { enterShop, shop } from "../lib/shop";
  import { filterValues } from "../lib/mapable";
  import { ChevronDownOutline } from "flowbite-svelte-icons";

  let myshops = $derived(filterValues($shops, (g) => g.owner === $user?.id));
</script>

{#if !$shop}
  <DropdownDivider />
  <DropdownItem href="/#/shops">Shops</DropdownItem>
  {#if myshops.length == 1}
    <DropdownItem onclick={() => enterShop(myshops[0].id!)}>
      Enter {myshops[0].name}
    </DropdownItem>
  {:else if myshops.length > 1}
    <DropdownItem class="cursor-pointer flex-end">
      Enter shop
      <ChevronDownOutline class=" inline " />
    </DropdownItem>
    <Dropdown simple>
      {#each myshops as shop}
        <DropdownItem onclick={() => enterShop(shop.id!)}>
          {shop.name}
        </DropdownItem>
      {/each}
    </Dropdown>
  {/if}
{/if}
