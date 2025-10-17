<script lang="ts">
  import { Button } from "flowbite-svelte";

  let { onclick, children } = $props();
</script>

<Button size="xs" color="alternative" class="p-1 cursor-pointer" {onclick}>
  {@render children()}
</Button>
