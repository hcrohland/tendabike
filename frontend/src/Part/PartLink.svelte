<script lang="ts">
  import { link } from "svelte-spa-router";
  import { Part } from "../lib/part";

  export let part: Part | undefined;
</script>

{#if part}
  <a
    href="/part/{part.id}"
    use:link
    style={part.disposed_at ? "text-decoration: line-through;" : ""}
    class="text-reset"
  >
    {part.name}
  </a>
{:else}
  -
{/if}
