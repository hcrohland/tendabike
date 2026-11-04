<script lang="ts">
  import { Part } from "../lib/part";

  interface Props {
    part: Part | undefined;
  }

  let { part }: Props = $props();
</script>

{#if part}
  <a
    href="/part/{part.id}"
    style={part.disposed_at ? "text-decoration: line-through;" : ""}
    class="text-reset"
  >
    {part.name}
  </a>
{:else}
  -
{/if}
