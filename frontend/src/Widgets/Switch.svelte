<script lang="ts">
  import { InputAddon, Toggle } from "flowbite-svelte";
  import type { RestoreWindowOutline } from "flowbite-svelte-icons";
  interface Props {
    checked: boolean;
    id?: string;
    children?: import("svelte").Snippet;
    class?: string;
  }

  let {
    checked = $bindable(),
    id = "custominputneedsone",
    children,
    ...rest
  }: Props = $props();
</script>

<Toggle
  size="small"
  {id}
  name="customSwitch"
  classes={{ span: "-z-10" }}
  bind:checked
  {...rest}
>
  {@render children?.()}
</Toggle>
