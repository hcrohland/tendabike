<script lang="ts">
  import { Alert } from "flowbite-svelte";
  import { parts } from "../lib/part";
  import Part from "./Part.svelte";

  export let params: { id: number };
</script>

{#if $parts[params.id]}
  <Part {...params} />
{:else}
  <Alert color="red" class="text-center">
    <span class="font-medium">Part not found!</span>
    Did you play with the url?
  </Alert>
{/if}
