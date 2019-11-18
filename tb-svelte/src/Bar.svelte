<script>
  import {types, categories, category} from './store.js';
  import { onMount, getContext } from 'svelte';

  import {
    Collapse,
    Navbar,
    NavbarToggler,
    NavbarBrand,
    Nav,
    NavItem,
    NavLink,
    UncontrolledDropdown,
    DropdownToggle,
    DropdownMenu,
    DropdownItem
  } from 'sveltestrap';
  import { Button, ButtonGroup } from "sveltestrap";

  let isOpen = false;

  function handleUpdate(event) {
    isOpen = event.detail.isOpen;
  }
</script>

<style>
    .active {
        background-color: lightgray;
    }
    button {
        padding-left: 2rem;
        padding-right: 2rem;
    }
</style>
<Nav expand="md">
  <!-- <NavbarBrand href="/">TendABike</NavbarBrand> -->
  <!-- <NavbarToggler on:click={() => (isOpen = !isOpen)} /> -->
  <!-- <Collapse {isOpen} navbar expand="md" on:update={handleUpdate}> -->
    <Nav class="nav-tabs">
    {#each $categories as cat}
            <NavItem>
                <button class="navlink" class:active="{$category === cat}" on:click="{() => $category = cat}">
                   <h3>
                     {$types[cat].name}s
                   </h3>
                </button>
            </NavItem>
        {/each}
    </Nav>
    <!-- <Nav class="ml-auto">
      <UncontrolledDropdown nav inNavbar>
        <DropdownToggle nav caret>Options</DropdownToggle>
        <DropdownMenu right>
          <DropdownItem>Option 1</DropdownItem>
          <DropdownItem>Option 2</DropdownItem>
          <DropdownItem divider />
          <DropdownItem>Reset</DropdownItem>
        </DropdownMenu>
      </UncontrolledDropdown>
    </Nav> -->
  <!-- </Collapse> -->
</Nav>