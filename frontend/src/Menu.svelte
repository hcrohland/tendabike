<Dropdown style="position:absolute" {isOpen} toggle={() => (isOpen = !isOpen)} size="sm">
  <DropdownToggle caret/>
  <DropdownMenu right>
    <slot />
  </DropdownMenu>
</Dropdown>

<script>
import {Dropdown, DropdownToggle, DropdownMenu, ButtonDropdown} from 'sveltestrap'

let isOpen

</script>