<script lang="ts">
  import {
    Button,
    ButtonGroup,
    Dropdown,
    DropdownItem,
    Input,
    InputAddon,
  } from "flowbite-svelte";
  import { Limits, type limit_keys } from "../lib/serviceplan";
  import { CircleMinusOutline, CirclePlusOutline } from "flowbite-svelte-icons";

  interface Props {
    select: any;
  }

  let { select = $bindable() }: Props = $props();

  let selected = $state(
    Object.entries(select || {})
      .filter(([k, v]) => Limits.keys.includes(k as any) && v != null)
      .map(([k, _]) => k as limit_keys),
  );
</script>

<div>
  {#each selected as key}
    <div>
      <ButtonGroup class="w-full">
        <Input
          type="number"
          placeholder={key}
          bind:value={select[key]}
          required
        />
        <InputAddon>{key}</InputAddon>
        <Button
          color="light"
          onclick={() => {
            select[key] = null;
            selected = selected.filter((k) => k != key);
          }}
        >
          <CircleMinusOutline class="shrink-0 h-6 w-6 justify-end" />
        </Button>
      </ButtonGroup>
    </div>
  {/each}
</div>
<Button color="alternative">
  <CirclePlusOutline class="shrink-0 h-6 w-6 justify-end" />
</Button>
<Dropdown>
  {#each Limits.keys.filter((k) => !selected.includes(k)) as key}
    <DropdownItem
      onclick={() => {
        selected.push(key);
        selected = selected;
      }}
    >
      {key}
    </DropdownItem>
  {/each}
</Dropdown>
