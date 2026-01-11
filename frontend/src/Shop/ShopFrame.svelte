<script lang="ts">
  import { Button } from "flowbite-svelte";
  import { exitShop, shop } from "../lib/shop";

  let { children } = $props();
</script>

<div class={$shop != undefined ? "border border-gray p-4 rounded-xl" : ""}>
  {#if $shop}
    <Button onclick={exitShop} color="gray" class="mb-8">
      Exit shop {$shop!.name}
    </Button>
    {@render children()}
  {:else}
    {@render children()}
  {/if}
</div>
