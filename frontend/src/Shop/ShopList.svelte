<script lang="ts">
  import {
    Button,
    Clipboard,
    Dropdown,
    DropdownItem,
    Input,
    Tooltip,
  } from "flowbite-svelte";
  import ShopCard from "./ShopCard.svelte";
  import { type Shop } from "../lib/shop";
  import { actions } from "../Widgets/Actions.svelte";
  import { user } from "../lib/store";
  import type { Snippet } from "svelte";
  import {
    CheckOutline,
    ClipboardCleanSolid,
    DotsVerticalOutline,
  } from "flowbite-svelte-icons";

  interface Props {
    shops: Shop[];
    sub?: Snippet<[Shop]>;
  }

  let { sub, shops }: Props = $props();
</script>

<div class="grid gap-4 grid-cols-1">
  {#each shops as shop}
    {@const isOwner = shop.owner === $user?.id}
    <ShopCard {shop} {isOwner} {sub}>
      {#if isOwner}
        <DotsVerticalOutline class="cursor-pointer" />
        <Dropdown>
          <DropdownItem onclick={() => $actions.editShop(shop)}>
            Edit Shop
          </DropdownItem>
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
      {:else}
        <Button onclick={() => $actions.requestSubscription(shop)}>
          Request Subscription
        </Button>
      {/if}
    </ShopCard>
  {/each}
</div>

{#if shops.length === 0}
  <div class="py-12 text-center">
    <p class="text-gray-500 dark:text-gray-400">
      No shops found. Create your first shop to get started!
    </p>
  </div>
{/if}
