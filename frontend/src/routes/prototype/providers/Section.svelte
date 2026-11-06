<!-- PROTOTYPE (throwaway): a section caption. A trivial primitive (not a
     layout) so the variants can focus on their real structural differences. -->
<script lang="ts">
  let { n = "", title }: { n?: string; title: string } = $props();
</script>

<p class="text-xs font-semibold uppercase tracking-wide text-text-1 mt-6 mb-2">
  {#if n}{n} ·
  {/if}{title}
</p>
