<script lang="ts">
  import { ButtonGroup, Input, InputAddon } from "flowbite-svelte";

  let { field = $bindable(), label }: { field?: number; label: string } =
    $props();

  let required = field != undefined;
</script>

<ButtonGroup class="w-full">
  <InputAddon class="w-1/2">{label}</InputAddon>
  <Input bind:value={field} type="number" min="0" {required} class="w-full" />
</ButtonGroup>
