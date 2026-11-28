<script lang="ts">
  import { Button } from "flowbite-svelte";
  import { fillBg } from "./color.js";

  let { children, ...rest } = $props();
</script>

<Button
  size="xs"
  color="secondary"
  class={"py-0 px-1 ms-2 rounded-xs cursor-pointer " + fillBg}
  {...rest}
>
  {@render children()}
</Button>
