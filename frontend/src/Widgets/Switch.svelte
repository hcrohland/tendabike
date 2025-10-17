<script lang="ts">
  import { InputAddon, Toggle } from "flowbite-svelte";
  interface Props {
    checked: boolean;
    id?: string;
    children?: import("svelte").Snippet;
  }

  let {
    checked = $bindable(),
    id = "custominputneedsone",
    children,
  }: Props = $props();
</script>

<Toggle size="small" {id} name="customSwitch" bind:checked>
  {@render children?.()}
</Toggle>
