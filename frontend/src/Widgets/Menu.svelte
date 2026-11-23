<script>
  import { Dropdown } from "flowbite-svelte";
  import { DotsVerticalOutline } from "flowbite-svelte-icons";
  /**
   * @typedef {Object} Props
   * @property {import('svelte').Snippet} [children]
   */

  /** @type {Props} */
  let { children } = $props();

  // The Dropdown (and its floating-ui popper) is instantiated on first
  // trigger interaction, not at mount: in long card lists the per-card
  // popper tree is the dominant cost of mounting and of DOM
  // reconciliation, and most cards' menus are never opened.
  let ready = $state(false);
  let isOpen = $state(false);

  // Mount the Dropdown in the same gesture as the first interaction. The
  // Dropdown's own trigger wiring attaches to the icon only after this
  // event has been dispatched, so it cannot handle this one — `isOpen`
  // does. Subsequent open/close is handled by the Dropdown itself.
  function onFirstInteraction() {
    if (!ready) {
      ready = true;
      isOpen = true;
    }
  }
</script>

<div>
  <DotsVerticalOutline
    class="cursor-pointer"
    onmousedown={onFirstInteraction}
  />
  {#if ready}
    <Dropdown simple bind:isOpen>
      {@render children?.()}
    </Dropdown>
  {/if}
</div>
