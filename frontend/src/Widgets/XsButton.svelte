<script lang="ts">
  import { Button } from "flowbite-svelte";

  let { children, ...rest } = $props();
</script>

<Button
  size="xs"
  color="alternative"
  class="py-0 px-1 ms-2 rounded-xs cursor-pointer"
  {...rest}
>
  {@render children()}
</Button>
