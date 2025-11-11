<script lang="ts">
  import { Toggle } from "flowbite-svelte";
  interface Props {
    checked: boolean;
    children?: import("svelte").Snippet;
    class?: string;
  }

  let { checked = $bindable(), children, ...rest }: Props = $props();
</script>

<Toggle size="small" bind:checked {...rest}>
  {@render children?.()}
</Toggle>
