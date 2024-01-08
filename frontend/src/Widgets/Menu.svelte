<div class="float-end">
  <Dropdown {isOpen} toggle={() => (isOpen = !isOpen)} size="sm">
    <DropdownToggle color=light caret/>
    <DropdownMenu end>
      <slot />
    </DropdownMenu>
  </Dropdown>
</div>

<script>
import {Dropdown, DropdownToggle, DropdownMenu} from '@sveltestrap/sveltestrap'

/**
 * @type boolean
 */
let isOpen;

</script>