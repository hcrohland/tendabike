<script>
  import { Dropdown } from "flowbite-svelte";
  import { ChevronDownOutline } from "flowbite-svelte-icons";
  /**
   * @typedef {Object} Props
   * @property {import('svelte').Snippet} [children]
   */

  /** @type {Props} */
  let { children } = $props();
</script>

<div>
  <ChevronDownOutline class="cursor-pointer" />
  <Dropdown simple>
    {@render children?.()}
  </Dropdown>
</div>
