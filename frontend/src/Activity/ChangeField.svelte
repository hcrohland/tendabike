<script lang="ts">
  import { ButtonGroup, Input, InputAddon } from "flowbite-svelte";

  export let field: number | undefined;
  export let label: string;

  let required = field != undefined;
</script>

<ButtonGroup class="w-full">
  <InputAddon class="w-1/2">{label}</InputAddon>
  <Input bind:value={field} type="number" min="0" {required} class="w-full" />
</ButtonGroup>
