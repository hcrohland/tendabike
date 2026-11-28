<script lang="ts">
  import {
    ButtonGroup,
    Dropdown,
    DropdownItem,
    Input,
    InputAddon,
  } from "flowbite-svelte";
  import {
    Limits,
    localizeLimitKey,
    type limit_keys,
  } from "../lib/serviceplan";
  import { CircleMinusOutline, CirclePlusOutline } from "flowbite-svelte-icons";
  import NeutralButton from "../Widgets/NeutralButton.svelte";

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
          placeholder={localizeLimitKey(key)}
          bind:value={select[key]}
          required
        />
        <InputAddon>{localizeLimitKey(key)}</InputAddon>
        <NeutralButton
          size="sm"
          onclick={() => {
            select[key] = null;
            selected = selected.filter((k) => k != key);
          }}
        >
          <CircleMinusOutline class="shrink-0 h-6 w-6 justify-end" />
        </NeutralButton>
      </ButtonGroup>
    </div>
  {/each}
</div>
<NeutralButton>
  <CirclePlusOutline class="shrink-0 h-6 w-6 justify-end" />
</NeutralButton>
<Dropdown>
  {#each Limits.keys.filter((k) => !selected.includes(k)) as key}
    <DropdownItem
      onclick={() => {
        selected.push(key);
        selected = selected;
      }}
    >
      {localizeLimitKey(key)}
    </DropdownItem>
  {/each}
</Dropdown>
