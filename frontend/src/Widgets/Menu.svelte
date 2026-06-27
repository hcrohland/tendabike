<script>
  import { Dropdown } from "flowbite-svelte";
  import { DotsVerticalOutline } from "flowbite-svelte-icons";
  /**
   * @typedef {Object} Props
   * @property {import('svelte').Snippet} [children]
   */

  /** @type {Props} */
  let { children } = $props();
</script>

<div>
  <DotsVerticalOutline class="cursor-pointer" />
  <Dropdown simple>
    {@render children?.()}
  </Dropdown>
</div>
