<script lang="ts">
  import { Button } from "flowbite-svelte";
  import * as m from "../../paraglide/messages";
  import { exitShop, getShop } from "../lib/shop";

  let { children } = $props();
</script>

<div class={getShop() != undefined ? "border border-gray p-4 rounded-xl" : ""}>
  {#if getShop()}
    <Button onclick={exitShop} color="gray" class="mb-8">
      {m.shop_exit({ name: getShop()!.name })}
    </Button>
    {@render children()}
  {:else}
    {@render children()}
  {/if}
</div>
