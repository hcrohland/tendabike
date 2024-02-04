<script>
  import {
    Dropdown,
    DropdownToggle,
    DropdownMenu,
  } from "@sveltestrap/sveltestrap";

  /**
   * @type boolean
   */
  let isOpen;
</script>

<div class="float-end">
  <Dropdown
    {isOpen}
    toggle={() => (isOpen = !isOpen)}
    size="sm"
    style="position: absolute;"
  >
    <DropdownToggle color="light" caret />
    <DropdownMenu end>
      <slot />
    </DropdownMenu>
  </Dropdown>
</div>
