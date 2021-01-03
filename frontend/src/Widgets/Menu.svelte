<Dropdown {isOpen} toggle={() => (isOpen = !isOpen)} size="sm" class="float-right">
  <DropdownToggle caret/>
  <DropdownMenu right>
    <slot />
  </DropdownMenu>
</Dropdown>

<script>
import {Dropdown, DropdownToggle, DropdownMenu} from 'sveltestrap'

let isOpen

</script>