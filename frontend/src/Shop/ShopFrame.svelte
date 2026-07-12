<script lang="ts">
  import { Button } from "flowbite-svelte";
  import * as m from "../../paraglide/messages";
  import { exitShop, shop } from "../lib/shop";

  let { children } = $props();
</script>

<div class={$shop != undefined ? "border border-gray p-4 rounded-xl" : ""}>
  {#if $shop}
    <Button onclick={exitShop} color="gray" class="mb-8">
      {m.shop_exit({ name: $shop.name })}
    </Button>
    {@render children()}
  {:else}
    {@render children()}
  {/if}
</div>
