<script lang="ts">
  import { Button } from "flowbite-svelte";

  let { children, ...rest } = $props();
</script>

<Button
  size="xs"
  color="alternative"
  class="p-1 rounded-md cursor-pointer"
  {...rest}
>
  {@render children()}
</Button>
