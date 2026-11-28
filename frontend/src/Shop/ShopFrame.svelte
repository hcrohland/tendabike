<script lang="ts">
  import * as m from "../../paraglide/messages";
  import { exitShop, getShop } from "../lib/shop.svelte";
  import NeutralButton from "../Widgets/NeutralButton.svelte";

  let { children = () => {} } = $props();
</script>

<div
  class={getShop() != undefined
    ? "border border-secondary-400 dark:border-secondary-700 p-4 rounded-xl"
    : ""}
>
  {#if getShop()}
    <NeutralButton onclick={exitShop} class="mb-8">
      {m.shop_exit({ name: getShop()!.name })}
    </NeutralButton>
    {@render children()}
  {:else}
    {@render children()}
  {/if}
</div>
