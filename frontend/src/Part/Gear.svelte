<script lang="ts">
  import { Alert } from "flowbite-svelte";
  import { parts } from "../lib/part";
  import Part from "./Part.svelte";
  import { m } from "../../paraglide/messages";

  export let params: { id: number };
</script>

{#if $parts[params.id]}
  <Part {...params} />
{:else}
  <Alert color="red" class="text-center">
    <span class="font-medium">{m.gear_not_found()}</span>
    {m.gear_not_found_hint()}
  </Alert>
{/if}
